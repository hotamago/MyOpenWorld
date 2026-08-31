//! Hợp đồng của bus — dùng lại nguyên vẹn cho NATS `JetStream` ở `PC-20`.
//!
//! Bộ này định nghĩa **ngữ nghĩa tối thiểu bắt buộc**. Điều quan trọng không
//! kém là những gì nó *không* kiểm: thứ tự giữa hai chủ đề khác nhau, công bằng
//! giữa nhiều consumer, độ trễ. Code gọi không được dựa vào những thứ đó, kể cả
//! khi hiện thực `SQLite` tình cờ cung cấp — vì `JetStream` sẽ không.

use crate::MessageBus;

/// Chạy toàn bộ hợp đồng.
pub fn run_all<B: MessageBus, F: Fn() -> B>(factory: F) {
    gui_roi_lay_lai(&factory);
    thu_tu_trong_mot_chu_de(&factory);
    chu_de_khac_nhau_khong_lan(&factory);
    lay_roi_khong_lay_lai_duoc(&factory);
    ack_roi_thi_bien_mat(&factory);
    nack_thi_quay_lai_hang_doi(&factory);
    recover_tra_lai_thu_dang_giu(&factory);
    delivery_count_tang_moi_lan_giao(&factory);
    ack_hai_lan_la_loi(&factory);
    payload_la_byte_duc(&factory);
}

/// Gửi rồi lấy lại được đúng nội dung.
pub fn gui_roi_lay_lai<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    let seq = b.publish("cognition.request", b"xin chao").unwrap();
    let m = b.fetch("cognition.request", 10).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].seq, seq);
    assert_eq!(m[0].payload, b"xin chao");
    assert_eq!(m[0].subject, "cognition.request");
}

/// Trong **một** chủ đề, thứ tự phải là thứ tự gửi.
///
/// Chỉ trong một chủ đề. Giữa hai chủ đề thì không có lời hứa nào, và code gọi
/// không được giả định có.
pub fn thu_tu_trong_mot_chu_de<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    for i in 0..5u8 {
        b.publish("s", &[i]).unwrap();
    }
    let m = b.fetch("s", 10).unwrap();
    assert_eq!(
        m.iter().map(|x| x.payload[0]).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4],
        "hợp đồng: thứ tự trong một chủ đề phải là thứ tự gửi"
    );
}

/// Chủ đề khác nhau không thấy nhau.
pub fn chu_de_khac_nhau_khong_lan<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("a", b"1").unwrap();
    b.publish("b", b"2").unwrap();
    assert_eq!(b.fetch("a", 10).unwrap().len(), 1);
    assert_eq!(b.fetch("b", 10).unwrap().len(), 1);
}

/// Đã lấy thì không lấy lại được cho tới khi nack hoặc recover.
///
/// Không có tính chất này thì hai vòng lặp consumer sẽ xử lý cùng một proposal.
pub fn lay_roi_khong_lay_lai_duoc<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"x").unwrap();
    assert_eq!(b.fetch("s", 10).unwrap().len(), 1);
    assert!(
        b.fetch("s", 10).unwrap().is_empty(),
        "hợp đồng: thông điệp đang giữ không được giao lần hai"
    );
}

/// Ack rồi thì biến mất vĩnh viễn.
pub fn ack_roi_thi_bien_mat<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"x").unwrap();
    let m = b.fetch("s", 1).unwrap();
    b.ack(m[0].seq).unwrap();
    assert_eq!(b.pending("s").unwrap(), 0);
    b.recover().unwrap();
    assert!(
        b.fetch("s", 10).unwrap().is_empty(),
        "hợp đồng: đã ack thì recover cũng không được trả lại"
    );
}

/// Nack thì quay lại hàng đợi ngay.
pub fn nack_thi_quay_lai_hang_doi<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"x").unwrap();
    let m = b.fetch("s", 1).unwrap();
    b.nack(m[0].seq).unwrap();
    assert_eq!(
        b.fetch("s", 10).unwrap().len(),
        1,
        "hợp đồng: nack phải trả thông điệp về hàng đợi"
    );
}

/// **Bài quan trọng nhất**: crash không làm mất proposal.
///
/// Mô phỏng crash bằng cách lấy thông điệp rồi không ack, sau đó `recover` như
/// lúc tiến trình khởi động lại. Nếu bài này fail thì một NPC đã suy nghĩ, kết
/// quả đã về, và hành động biến mất không dấu vết.
pub fn recover_tra_lai_thu_dang_giu<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"proposal").unwrap();
    let lay = b.fetch("s", 1).unwrap();
    assert_eq!(lay.len(), 1);
    // ... tiến trình chết ở đây, không ack ...

    let n = b.recover().unwrap();
    assert_eq!(n, 1, "hợp đồng: recover phải trả lại đúng thứ đang giữ");

    let lai = b.fetch("s", 10).unwrap();
    assert_eq!(lai.len(), 1, "hợp đồng: proposal không được mất khi crash");
    assert_eq!(lai[0].payload, b"proposal");
}

/// Số lần giao tăng dần, để phát hiện thông điệp độc.
pub fn delivery_count_tang_moi_lan_giao<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"x").unwrap();
    for mong_doi in 1..=3u32 {
        let m = b.fetch("s", 1).unwrap();
        assert_eq!(
            m[0].delivery_count, mong_doi,
            "hợp đồng: delivery_count phải tăng mỗi lần giao"
        );
        b.nack(m[0].seq).unwrap();
    }
}

/// Ack hai lần là lỗi, không phải chuyện im lặng bỏ qua.
pub fn ack_hai_lan_la_loi<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    b.publish("s", b"x").unwrap();
    let m = b.fetch("s", 1).unwrap();
    b.ack(m[0].seq).unwrap();
    assert!(
        b.ack(m[0].seq).is_err(),
        "hợp đồng: ack lần hai phải báo lỗi — nó là dấu hiệu consumer xử lý hai lần"
    );
}

/// Payload không bị diễn giải.
pub fn payload_la_byte_duc<B: MessageBus, F: Fn() -> B>(f: &F) {
    let mut b = f();
    let tho = vec![0u8, 0xff, 0x00, 0x80];
    b.publish("s", &tho).unwrap();
    let m = b.fetch("s", 1).unwrap();
    assert_eq!(m[0].payload, tho);
}
