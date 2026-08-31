//! Hiện thực thứ hai của [`MessageBus`]: NATS JetStream (`plan.md §P3.4`, `PC-20`).
//!
//! ## Vì sao dùng client bất đồng bộ cho một trait đồng bộ
//!
//! Client `nats` đồng bộ đã ngừng bảo trì, và nó hỏng theo một cách rất cụ thể:
//! phụ thuộc bắc cầu `nuid 0.3.2` khai `rand = ">=0.8"`, nên Cargo kéo `rand`
//! 0.10 và `nuid` không biên dịch được với chính thứ nó xin. Đó không phải lỗi
//! sửa được ở đây.
//!
//! Nên backend này dùng `async-nats` và **tự giữ một runtime**. Ranh giới nằm
//! gọn trong file này: trait vẫn đồng bộ, bản desktop vẫn không biết tokio tồn
//! tại, và chỉ server mode trả cái giá đó.
//!
//! `block_on` ở đây an toàn vì [`MessageBus`] được gọi từ luồng mô phỏng, vốn
//! không nằm trong một runtime async nào — `block_on` bên trong một runtime mới
//! là thứ gây panic, và điều đó không xảy ra trên đường này.
//!
//! ## Ánh xạ ngữ nghĩa, và chỗ nó không khớp
//!
//! ```text
//!  MessageBus            JetStream
//!  ──────────────────────────────────────────────
//!  publish        →      js.publish
//!  fetch          →      consumer.fetch (pull, ack thủ công)
//!  ack            →      msg.ack
//!  nack           →      msg.ack_with(Nak)   (giao lại ngay)
//!  recover        →      KHÔNG có tương đương trực tiếp
//!  pending        →      num_pending + num_ack_pending
//! ```
//!
//! [`MessageBus::recover`] là chỗ hai mô hình khác nhau thật sự.
//!
//! Bus SQLite giữ trạng thái `LEASED` **trong bảng**, nên một tiến trình chết để
//! lại thông điệp treo vĩnh viễn cho tới khi ai đó gọi `recover()`. JetStream
//! không cần: nó có `ack_wait`, và thông điệp không được ack trong khoảng đó
//! **tự** quay lại hàng đợi. Ở backend này, `recover()` là chuyện đã xảy ra sẵn.
//!
//! Điều đó **không** làm bộ test hợp đồng sai — nó làm lộ ra một điều đáng giá:
//! hợp đồng phải được viết theo *quan sát được* ("sau khi khôi phục, thông điệp
//! lấy lại được") chứ không theo con số trả về. Nếu nó khẳng định
//! `recover() == 2` thì hợp đồng đã bị viết quanh SQLite, và backend thứ hai sẽ
//! đỏ vì một lý do không có nghĩa gì với người dùng.
//!
//! ## Chạy test hợp đồng
//!
//! ```bash
//! ./mow infra up
//! MOWTEST_NATS_URL=nats://localhost:14222 \
//!   cargo test -p mow-bus --features nats -- --ignored
//! ```

use crate::{BusError, BusResult, Message, MessageBus};
use async_nats::jetstream::{self, consumer::PullConsumer};
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;

fn loi(e: impl std::fmt::Display) -> BusError {
    BusError::External(e.to_string())
}

/// Bus trên NATS `JetStream`.
pub struct NatsBus {
    rt: Runtime,
    js: jetstream::Context,
    stream: String,
    /// `seq` → thông điệp đang giữ.
    ///
    /// Cần bảng này vì hợp đồng nói *"ack theo `seq`"* còn `JetStream` nói *"ack
    /// theo đối tượng message"*. Không giữ lại thì `ack(seq)` không có gì để
    /// gọi, và cách vá rẻ — ack theo thứ tự nhận — sẽ sai ngay lần đầu có ai đó
    /// ack không theo thứ tự.
    dang_giu: HashMap<u64, jetstream::Message>,
}

impl NatsBus {
    /// Kết nối và tạo stream nếu chưa có.
    pub fn connect(url: &str, stream: &str) -> BusResult<NatsBus> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(loi)?;

        let (js, ()) = rt.block_on(async {
            let nc = async_nats::connect(url).await.map_err(loi)?;
            let js = jetstream::new(nc);
            // `WorkQueue`: mỗi thông điệp được **một** consumer xử lý và biến
            // mất sau khi ack. Đó đúng là ngữ nghĩa của `MessageBus` ở đây — một
            // hàng đợi công việc, không phải một luồng sự kiện phát cho nhiều bên.
            js.get_or_create_stream(jetstream::stream::Config {
                name: stream.to_owned(),
                subjects: vec![format!("{stream}.>")],
                retention: jetstream::stream::RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await
            .map_err(loi)?;
            Ok::<_, BusError>((js, ()))
        })?;

        Ok(NatsBus {
            rt,
            js,
            stream: stream.to_owned(),
            dang_giu: HashMap::new(),
        })
    }

    fn chu_de(&self, subject: &str) -> String {
        format!("{}.{subject}", self.stream)
    }

    /// Tên durable phải hợp lệ với NATS: không dấu chấm.
    fn ten_consumer(subject: &str) -> String {
        format!("c_{}", subject.replace('.', "_"))
    }

    async fn consumer(&self, subject: &str) -> BusResult<PullConsumer> {
        let s = self.js.get_stream(&self.stream).await.map_err(loi)?;
        s.get_or_create_consumer(
            &NatsBus::ten_consumer(subject),
            jetstream::consumer::pull::Config {
                durable_name: Some(NatsBus::ten_consumer(subject)),
                filter_subject: self.chu_de(subject),
                // Đủ lâu để một chu trình nhận thức chạy xong. Quá ngắn thì
                // thông điệp bị giao lại trong lúc còn đang được xử lý, và cùng
                // một proposal chạy hai lần.
                ack_wait: Duration::from_secs(60),
                ..Default::default()
            },
        )
        .await
        .map_err(loi)
    }
}

impl MessageBus for NatsBus {
    fn publish(&mut self, subject: &str, payload: &[u8]) -> BusResult<u64> {
        let chu_de = self.chu_de(subject);
        let data = payload.to_vec();
        let js = self.js.clone();
        self.rt.block_on(async move {
            let ack = js.publish(chu_de, data.into()).await.map_err(loi)?;
            // Chờ xác nhận: hợp đồng nói *"khi `publish` trả `Ok` thì thông điệp
            // đã bền"*. Không chờ thì hàm trả về trước khi máy chủ ghi xong, và
            // lời hứa đó thành sai.
            Ok(ack.await.map_err(loi)?.sequence)
        })
    }

    fn fetch(&mut self, subject: &str, max: usize) -> BusResult<Vec<Message>> {
        let ten = subject.to_owned();
        let (ra, giu) = self.rt.block_on(async {
            let con = self.consumer(&ten).await?;
            let mut batch = con
                .fetch()
                .max_messages(max)
                .messages()
                .await
                .map_err(loi)?;

            let mut ra = Vec::new();
            let mut giu = Vec::new();
            while let Some(m) = batch.next().await {
                let m = m.map_err(loi)?;
                let info = m.info().map_err(loi)?;
                let seq = info.stream_sequence;
                ra.push(Message {
                    seq,
                    subject: ten.clone(),
                    payload: m.payload.to_vec(),
                    // JetStream đếm từ 1 cho lần giao đầu, giống bus SQLite.
                    delivery_count: u32::try_from(info.delivered).unwrap_or(u32::MAX),
                });
                giu.push((seq, m));
            }
            Ok::<_, BusError>((ra, giu))
        })?;

        self.dang_giu.extend(giu);
        Ok(ra)
    }

    fn ack(&mut self, seq: u64) -> BusResult<()> {
        let m = self.dang_giu.remove(&seq).ok_or(BusError::NotLeased(seq))?;
        self.rt.block_on(async { m.ack().await.map_err(loi) })
    }

    fn nack(&mut self, seq: u64) -> BusResult<()> {
        let m = self.dang_giu.remove(&seq).ok_or(BusError::NotLeased(seq))?;
        self.rt.block_on(async {
            // `Nak` yêu cầu giao lại **ngay**, không chờ hết `ack_wait`.
            m.ack_with(jetstream::AckKind::Nak(None)).await.map_err(loi)
        })
    }

    fn recover(&mut self) -> BusResult<usize> {
        // JetStream tự trả thông điệp quá `ack_wait` về hàng đợi, nên không có
        // gì để khôi phục thủ công. Bỏ tham chiếu tới những thông điệp tiến
        // trình này còn giữ; chúng sẽ được giao lại khi hết hạn.
        //
        // Trả `0` là **đúng**, không phải là chưa làm: không có thông điệp nào
        // bị mắc kẹt cần cứu. Xem phần đầu file.
        let n = self.dang_giu.len();
        let giu: Vec<jetstream::Message> = self.dang_giu.drain().map(|(_, m)| m).collect();
        // Nak ngay để lần `fetch` sau lấy lại được, thay vì đợi hết `ack_wait` —
        // hợp đồng nói "sau khi khôi phục, thông điệp lấy lại được", và chờ 60
        // giây thì về mặt quan sát là không lấy lại được.
        self.rt.block_on(async {
            for m in giu {
                let _ = m.ack_with(jetstream::AckKind::Nak(None)).await;
            }
        });
        Ok(n)
    }

    fn pending(&self, subject: &str) -> BusResult<usize> {
        let ten = subject.to_owned();
        self.rt.block_on(async {
            let mut con = self.consumer(&ten).await?;
            let info = con.info().await.map_err(loi)?;
            // Cả chưa giao lẫn đang giữ — hợp đồng nói "chưa ack", và một thông
            // điệp đang được xử lý vẫn là chưa xong.
            Ok(usize::try_from(info.num_pending).unwrap_or(usize::MAX) + info.num_ack_pending)
        })
    }
}
