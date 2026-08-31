//! Embedding — biến văn bản thành vector cho chỉ mục ký ức (`§11`, `§P6.3`).
//!
//! `mow-vector::quantize` nhận `&[f32]` "từ mô hình embedding bên ngoài". Cho
//! tới file này, **không có mô hình nào** — hàm đó có một tham số mà không có
//! ai truyền vào.
//!
//! ## Vì sao embedding nằm ở `mow-llm` chứ không ở `mow-vector`
//!
//! Vì nó là một lời gọi mô hình. `mow-vector` là chỉ mục: nó nhận vector đã
//! lượng tử hóa và trả về thứ hạng, và nó không nên biết HTTP là gì. Đặt client
//! ở đây giữ đúng biên mà `mow-vector` đã tuyên bố: số thực chỉ tồn tại ở phía
//! **nhận** dữ liệu, và biến mất ngay ở `quantize`.
//!
//! ## Hai hiện thực, và cái không cần mạng mới là cái quan trọng
//!
//! [`HashingEmbedder`] chạy được ngay, không khóa, không mạng, xác định tuyệt
//! đối. Nó dùng thủ thuật băm đặc trưng (Weinberger và cộng sự, 2009): mỗi từ
//! băm vào một ô, dấu cũng lấy từ băm để nhiễu triệt tiêu thay vì cộng dồn.
//!
//! Nó cho **tương đồng từ vựng, không phải tương đồng ngữ nghĩa** — và đó là
//! một khác biệt phải nói thẳng, vì nó là đúng cái khác biệt mà một bản demo dễ
//! che giấu. "Con ngựa chết" và "con tuấn mã qua đời" gần nhau về nghĩa và xa
//! nhau hoàn toàn ở đây. Cái nó mua được là: toàn bộ đường ống ký ức chạy được,
//! kiểm được, và **replay bit-perfect**, trước khi có bất kỳ khóa API nào.
//!
//! [`HttpEmbedder`] là bản thật, nói chuyện với mọi endpoint theo lược đồ
//! `OpenAI` — TEI, vLLM, `OpenAI`, `OpenRouter` — nên đổi chỗ phục vụ chỉ là đổi
//! `base_url`.
//!
//! ## Truy vấn và tài liệu không phải cùng một thứ
//!
//! Model truy xuất hiện đại được huấn luyện **bất đối xứng**: câu hỏi được mã
//! hóa theo một cách, đoạn văn được lưu theo một cách khác. `jina-embeddings-v5`
//! gọi đó là `prompt_name` (`query` / `document`).
//!
//! Dùng nhầm vai không làm gì hỏng ra mặt. Nó chỉ làm chất lượng truy xuất tụt
//! đi vài phần trăm — nghĩa là NPC thỉnh thoảng nhớ ra một ký ức hơi lệch, mãi
//! mãi, và không có bài test nào đỏ. Nên vai là **tham số bắt buộc** của
//! [`Embedder::embed`], không phải một tùy chọn có mặc định.

use crate::client::{LlmError, LlmResult};
use crate::provider::{che_bi_mat, Transport};
use mow_math::StateHasher;
use serde_json::{json, Value};

/// Vai của văn bản trong một tác vụ truy xuất.
///
/// Xem phần cuối tài liệu module: đây là tham số bắt buộc vì dùng nhầm nó là
/// một lỗi im lặng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmbedRole {
    /// Văn bản được cất vào chỉ mục.
    Document,
    /// Văn bản dùng để tìm.
    Query,
}

/// Nguồn sinh vector.
pub trait Embedder {
    /// Số chiều mà mọi vector trả về phải có.
    fn dimension(&self) -> usize;

    /// Mã hóa một lô văn bản.
    ///
    /// Thứ tự kết quả **luôn** khớp thứ tự đầu vào.
    fn embed(&self, role: EmbedRole, texts: &[&str]) -> LlmResult<Vec<Vec<f32>>>;
}

// ── Bản không cần mạng ───────────────────────────────────────────────────────

/// Băm đặc trưng: xác định, ngoại tuyến, tương đồng **từ vựng**.
///
/// Xem tài liệu module về việc nó mua được gì và không mua được gì.
#[derive(Debug, Clone)]
pub struct HashingEmbedder {
    dim: usize,
}

impl HashingEmbedder {
    /// Tạo với số chiều cho trước.
    ///
    /// # Panics
    /// Khi `dim` bằng 0 — một chỉ mục 0 chiều không phải một cấu hình lạ, nó là
    /// một lỗi lập trình.
    #[must_use]
    pub fn new(dim: usize) -> HashingEmbedder {
        assert!(dim > 0, "chỉ mục phải có ít nhất 1 chiều");
        HashingEmbedder { dim }
    }

    /// Tách từ: hạ chữ thường, cắt ở mọi ký tự không phải chữ-số.
    ///
    /// Đủ cho tiếng Việt có dấu vì `char::is_alphanumeric` theo Unicode chứ
    /// không theo ASCII — `"đường"` là một từ, không phải ba mảnh.
    fn tach_tu(s: &str) -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(str::to_lowercase)
            .collect()
    }

    fn bam(t: &str) -> u64 {
        let mut h = StateHasher::with_domain("mow.embed.hashing.v1");
        h.write_str(t);
        let b = h.finish().0;
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }

    fn mot_cau(&self, s: &str) -> Vec<f32> {
        // Cộng dồn bằng **số nguyên**. Không phải sự cầu kỳ: cộng `f32` không
        // kết hợp, nên thứ tự cộng đổi thì chữ số cuối đổi, và hai ký ức gần
        // bằng nhau có thể đảo thứ hạng. Ở đây phép thực duy nhất là bước chuẩn
        // hóa cuối cùng, tính từ một tổng nguyên — nên nó cho cùng bit trên mọi
        // máy.
        let mut o = vec![0i64; self.dim];
        for t in Self::tach_tu(s) {
            let h = Self::bam(&t);
            let i = (h % self.dim as u64) as usize;
            // Bit cao làm dấu: hai từ khác nhau rơi cùng ô thì triệt tiêu nhau
            // một nửa số lần thay vì luôn cộng dồn. Đây là toàn bộ lý do "signed
            // hashing" tồn tại.
            if (h >> 63) & 1 == 1 {
                o[i] -= 1;
            } else {
                o[i] += 1;
            }
        }

        let tong_binh_phuong: i128 = o.iter().map(|v| i128::from(*v) * i128::from(*v)).sum();
        if tong_binh_phuong == 0 {
            return vec![0.0; self.dim];
        }
        // allow-float: biên sinh vector, đúng chỗ mà `mow-vector` mô tả.
        #[allow(clippy::cast_precision_loss)]
        let chuan = (tong_binh_phuong as f64).sqrt();
        o.iter()
            .map(|v| {
                #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                {
                    (*v as f64 / chuan) as f32
                }
            })
            .collect()
    }
}

impl Embedder for HashingEmbedder {
    fn dimension(&self) -> usize {
        self.dim
    }

    fn embed(&self, role: EmbedRole, texts: &[&str]) -> LlmResult<Vec<Vec<f32>>> {
        // Vai không đổi kết quả ở đây, và điều đó là **đúng**: băm từ vựng
        // không có khái niệm bất đối xứng. Nhận tham số rồi bỏ qua nó vẫn tốt
        // hơn là không có tham số — vì chỗ gọi buộc phải nghĩ về vai, và khi
        // đổi sang `HttpEmbedder` thì không phải sửa một chỗ gọi nào.
        let _ = role;
        Ok(texts.iter().map(|t| self.mot_cau(t)).collect())
    }
}

// ── Bản thật ─────────────────────────────────────────────────────────────────

/// Client `/v1/embeddings` theo lược đồ `OpenAI`.
pub struct HttpEmbedder<T: Transport> {
    base: String,
    api_key: String,
    model: String,
    dim: usize,
    batch: usize,
    gui_dimensions: bool,
    tien_to_query: String,
    tien_to_document: String,
    transport: T,
}

impl<T: Transport> core::fmt::Debug for HttpEmbedder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HttpEmbedder")
            .field("base", &self.base)
            .field("model", &self.model)
            .field("dim", &self.dim)
            .field("co_khoa", &!self.api_key.is_empty())
            .finish_non_exhaustive()
    }
}

impl<T: Transport> HttpEmbedder<T> {
    /// Dựng client. `base` không kèm `/embeddings`.
    pub fn new(base: &str, api_key: &str, model: &str, dim: usize, transport: T) -> Self {
        HttpEmbedder {
            base: base.trim_end_matches('/').to_owned(),
            api_key: api_key.to_owned(),
            model: model.to_owned(),
            dim,
            batch: 32,
            gui_dimensions: true,
            tien_to_query: String::new(),
            tien_to_document: String::new(),
            transport,
        }
    }

    /// Số văn bản mỗi lời gọi.
    #[must_use]
    pub fn with_batch(mut self, n: usize) -> Self {
        self.batch = n.max(1);
        self
    }

    /// Có gửi trường `dimensions` hay không.
    ///
    /// Bật là đúng với model Matryoshka (`jina-embeddings-v5` cắt được về
    /// 32/64/128/256/512/768/1024). Tắt khi máy chủ từ chối trường này — TEI
    /// bản cũ trả `422` và thông báo của nó không nói rõ trường nào sai.
    #[must_use]
    pub fn with_send_dimensions(mut self, b: bool) -> Self {
        self.gui_dimensions = b;
        self
    }

    /// Tiền tố theo vai, nếu máy chủ không tự áp.
    #[must_use]
    pub fn with_prefixes(mut self, query: &str, document: &str) -> Self {
        query.clone_into(&mut self.tien_to_query);
        document.clone_into(&mut self.tien_to_document);
        self
    }

    fn tien_to(&self, role: EmbedRole) -> &str {
        match role {
            EmbedRole::Query => &self.tien_to_query,
            EmbedRole::Document => &self.tien_to_document,
        }
    }

    /// Thân yêu cầu cho một lô.
    fn than_yeu_cau(&self, role: EmbedRole, lo: &[&str]) -> Value {
        let tt = self.tien_to(role);
        let dau_vao: Vec<String> = lo.iter().map(|t| format!("{tt}{t}")).collect();
        let mut than = json!({
            "model": self.model,
            "input": dau_vao,
            // Tường minh: một số máy chủ mặc định `base64`, và một mảng base64
            // đọc thành mảng số sẽ cho một vector rỗng chứ không cho lỗi.
            "encoding_format": "float",
        });
        if self.gui_dimensions {
            than["dimensions"] = json!(self.dim);
        }
        than
    }

    /// Dịch một trả lời không phải `2xx` thành lỗi có ích.
    fn loi_upstream(&self, reply: &crate::provider::HttpReply) -> LlmError {
        let msg = serde_json::from_str::<Value>(&reply.body)
            .ok()
            .and_then(|v| {
                v.pointer("/error/message")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| reply.body.clone());
        // Gợi ý đúng chỗ cần sửa. `422` khi đang gửi `dimensions` gần như luôn
        // là máy chủ không hỗ trợ cắt Matryoshka, và thông báo gốc của nó
        // thường chỉ nói "unprocessable entity".
        let goi_y = if self.gui_dimensions && (reply.status == 422 || reply.status == 400) {
            ". Nếu máy chủ không hỗ trợ cắt Matryoshka, đặt \
             `embedding.send_dimensions: false` và để `vector.dimension` \
             bằng số chiều gốc của model"
        } else {
            ""
        };
        LlmError::Upstream {
            status: reply.status,
            message: format!("{}{goi_y}", che_bi_mat(&msg)),
        }
    }

    /// Đọc một phần tử `data` thành `(vị trí, vector)`.
    ///
    /// Đọc `index` chứ không đọc theo thứ tự trong mảng. Lược đồ `OpenAI`
    /// **không** hứa `data` giữ thứ tự đầu vào, và một máy chủ gộp lô song song
    /// có lý do rất chính đáng để trả về thứ tự khác. Đọc theo vị trí sẽ gán ký
    /// ức của người này cho người kia — im lặng, và không thể truy ngược.
    fn doc_mot_diem(&self, d: &Value, co_lo: usize) -> LlmResult<(usize, Vec<f32>)> {
        let i = usize::try_from(
            d.get("index")
                .and_then(Value::as_u64)
                .ok_or_else(|| LlmError::BadResponse("phần tử `data` thiếu `index`".to_owned()))?,
        )
        .map_err(|_| LlmError::BadResponse("`index` không hợp lệ".to_owned()))?;
        if i >= co_lo {
            return Err(LlmError::BadResponse(format!(
                "`index` {i} nằm ngoài lô {co_lo}"
            )));
        }
        let vec_f: Vec<f32> = d
            .get("embedding")
            .and_then(Value::as_array)
            .ok_or_else(|| LlmError::BadResponse("phần tử `data` thiếu `embedding`".to_owned()))?
            .iter()
            .map(|x| {
                #[allow(clippy::cast_possible_truncation)]
                {
                    x.as_f64().unwrap_or(0.0) as f32
                }
            })
            .collect();
        if vec_f.len() != self.dim {
            return Err(LlmError::BadResponse(format!(
                "model trả {} chiều, cấu hình khai {} chiều. Đặt `vector.dimension` \
                 bằng số chiều gốc của model, hoặc bật `embedding.send_dimensions` \
                 nếu model cắt được Matryoshka",
                vec_f.len(),
                self.dim
            )));
        }
        Ok((i, vec_f))
    }

    fn mot_lo(&self, role: EmbedRole, lo: &[&str]) -> LlmResult<Vec<Vec<f32>>> {
        let hdr = [
            ("Authorization", format!("Bearer {}", self.api_key)),
            ("Content-Type", "application/json".to_owned()),
        ];
        let reply = self
            .transport
            .post_json(
                &format!("{}/embeddings", self.base),
                &hdr,
                &self.than_yeu_cau(role, lo).to_string(),
            )
            .map_err(|e| LlmError::Transport(che_bi_mat(&e)))?;

        if !(200..300).contains(&reply.status) {
            return Err(self.loi_upstream(&reply));
        }

        let v: Value = serde_json::from_str(&reply.body)
            .map_err(|e| LlmError::BadResponse(format!("{e}: {}", che_bi_mat(&reply.body))))?;
        // Lỗi mang mã 200: cùng cái bẫy đã đóng ở `provider`.
        if let Some(e) = v.pointer("/error/message").and_then(Value::as_str) {
            return Err(LlmError::Upstream {
                status: 200,
                message: che_bi_mat(e),
            });
        }

        let data = v
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| LlmError::BadResponse("thiếu `data`".to_owned()))?;
        if data.len() != lo.len() {
            return Err(LlmError::BadResponse(format!(
                "gửi {} văn bản, nhận {} vector",
                lo.len(),
                data.len()
            )));
        }

        let mut ra: Vec<Option<Vec<f32>>> = vec![None; lo.len()];
        for d in data {
            let (i, vec_f) = self.doc_mot_diem(d, lo.len())?;
            ra[i] = Some(vec_f);
        }
        ra.into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.ok_or_else(|| LlmError::BadResponse(format!("thiếu vector cho vị trí {i}")))
            })
            .collect()
    }
}

impl<T: Transport> Embedder for HttpEmbedder<T> {
    fn dimension(&self) -> usize {
        self.dim
    }

    fn embed(&self, role: EmbedRole, texts: &[&str]) -> LlmResult<Vec<Vec<f32>>> {
        let mut ra = Vec::with_capacity(texts.len());
        for lo in texts.chunks(self.batch) {
            ra.extend(self.mot_lo(role, lo)?);
        }
        Ok(ra)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::HttpReply;
    use std::cell::RefCell;

    // ── HashingEmbedder ─────────────────────────────────────────────────────

    #[test]
    fn so_chieu_dung_va_chuan_hoa_l2() {
        let e = HashingEmbedder::new(64);
        let v = e
            .embed(EmbedRole::Document, &["con ngựa già ăn cỏ"])
            .unwrap();
        assert_eq!(v[0].len(), 64);
        let n: f64 = v[0].iter().map(|x| f64::from(*x) * f64::from(*x)).sum();
        assert!((n - 1.0).abs() < 1e-5, "chuẩn L2 = {n}");
    }

    #[test]
    fn xac_dinh_tuyet_doi_qua_nhieu_lan_goi() {
        let e = HashingEmbedder::new(128);
        let a = e.embed(EmbedRole::Document, &["mưa rơi trên mái"]).unwrap();
        let b = e.embed(EmbedRole::Document, &["mưa rơi trên mái"]).unwrap();
        assert_eq!(a, b, "cùng đầu vào phải cho cùng bit");
    }

    #[test]
    fn thu_tu_tu_khong_doi_ket_qua() {
        // Túi từ: đó là tính chất, không phải khiếm khuyết — và nó là lý do
        // phần tài liệu nói thẳng đây là tương đồng từ vựng.
        let e = HashingEmbedder::new(64);
        let a = e.embed(EmbedRole::Document, &["ngựa ăn cỏ"]).unwrap();
        let b = e.embed(EmbedRole::Document, &["cỏ ăn ngựa"]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn chia_tu_thi_gan_hon_khong_chia_tu() {
        let e = HashingEmbedder::new(512);
        let v = e
            .embed(
                EmbedRole::Document,
                &[
                    "người thợ rèn đúc một thanh kiếm",
                    "người thợ rèn đúc một cái cuốc",
                    "trận mưa sao băng tháng tám",
                ],
            )
            .unwrap();
        let cham = |a: &[f32], b: &[f32]| -> f64 {
            a.iter()
                .zip(b)
                .map(|(x, y)| f64::from(*x) * f64::from(*y))
                .sum()
        };
        let gan = cham(&v[0], &v[1]);
        let xa = cham(&v[0], &v[2]);
        assert!(
            gan > xa,
            "chia 4 từ ({gan}) phải hơn không chia từ nào ({xa})"
        );
    }

    #[test]
    fn chuoi_rong_cho_vector_khong_chuoi_khong_panic() {
        let e = HashingEmbedder::new(32);
        let v = e.embed(EmbedRole::Query, &["", "   ", "!!!"]).unwrap();
        assert_eq!(v.len(), 3);
        assert!(v.iter().all(|x| x.iter().all(|c| *c == 0.0)));
    }

    #[test]
    fn tieng_viet_co_dau_la_mot_tu() {
        let e = HashingEmbedder::new(256);
        let a = e.embed(EmbedRole::Document, &["đường"]).unwrap();
        let b = e.embed(EmbedRole::Document, &["đ ường"]).unwrap();
        assert_ne!(a, b, "`đường` không được vỡ thành mảnh");
    }

    #[test]
    fn hoa_thuong_khong_quan_trong() {
        let e = HashingEmbedder::new(64);
        let a = e.embed(EmbedRole::Document, &["Kiếm Sắt"]).unwrap();
        let b = e.embed(EmbedRole::Document, &["kiếm sắt"]).unwrap();
        assert_eq!(a, b);
    }

    // ── HttpEmbedder ────────────────────────────────────────────────────────

    struct Gia {
        tra_loi: RefCell<Vec<HttpReply>>,
        da_nhan: RefCell<Vec<String>>,
    }

    impl Gia {
        fn moi(cac: Vec<(u16, String)>) -> Gia {
            Gia {
                tra_loi: RefCell::new(
                    cac.into_iter()
                        .map(|(s, b)| HttpReply { status: s, body: b })
                        .collect(),
                ),
                da_nhan: RefCell::new(Vec::new()),
            }
        }
    }

    unsafe impl Send for Gia {}

    impl Transport for Gia {
        fn post_json(
            &self,
            _: &str,
            _: &[(&str, String)],
            body: &str,
        ) -> Result<HttpReply, String> {
            self.da_nhan.borrow_mut().push(body.to_owned());
            let mut ds = self.tra_loi.borrow_mut();
            if ds.is_empty() {
                return Err("hết kịch bản".to_owned());
            }
            Ok(ds.remove(0))
        }
    }

    fn than(cac: &[(usize, Vec<f32>)]) -> String {
        let data: Vec<Value> = cac
            .iter()
            .map(|(i, v)| json!({ "index": i, "object": "embedding", "embedding": v }))
            .collect();
        json!({ "object": "list", "model": "m", "data": data }).to_string()
    }

    #[test]
    fn doc_dung_vector_va_giu_thu_tu() {
        let g = Gia::moi(vec![(
            200,
            than(&[(0, vec![1.0, 0.0]), (1, vec![0.0, 1.0])]),
        )]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 2, g);
        let v = e.embed(EmbedRole::Document, &["a", "b"]).unwrap();
        assert_eq!(v, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    fn data_dao_thu_tu_van_gan_dung_cho() {
        // Máy chủ trả về ngược thứ tự. Đọc theo vị trí sẽ gán nhầm, và không
        // có gì báo.
        let g = Gia::moi(vec![(
            200,
            than(&[(1, vec![0.0, 1.0]), (0, vec![1.0, 0.0])]),
        )]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 2, g);
        let v = e.embed(EmbedRole::Document, &["a", "b"]).unwrap();
        assert_eq!(v, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    fn so_chieu_lech_thi_tu_choi_chu_khong_cat_bot() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0, 0.0, 0.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 2, g);
        let err = e.embed(EmbedRole::Document, &["a"]).unwrap_err();
        assert!(err.to_string().contains("3 chiều"), "{err}");
        assert!(err.to_string().contains("vector.dimension"), "{err}");
    }

    #[test]
    fn chia_lo_theo_batch() {
        let g = Gia::moi(vec![
            (200, than(&[(0, vec![1.0]), (1, vec![1.0])])),
            (200, than(&[(0, vec![1.0])])),
        ]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g).with_batch(2);
        let v = e.embed(EmbedRole::Document, &["a", "b", "c"]).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(e.transport.da_nhan.borrow().len(), 2, "phải gửi 2 lô");
    }

    #[test]
    fn tien_to_theo_vai_duoc_ap() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g)
            .with_prefixes("truy vấn: ", "tài liệu: ");
        e.embed(EmbedRole::Query, &["cái giếng ở đâu"]).unwrap();
        let b = e.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&b[0]).unwrap();
        assert_eq!(v["input"][0], "truy vấn: cái giếng ở đâu");
    }

    #[test]
    fn vai_khac_nhau_thi_tien_to_khac_nhau() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g)
            .with_prefixes("truy vấn: ", "tài liệu: ");
        e.embed(EmbedRole::Document, &["cái giếng ở đâu"]).unwrap();
        let b = e.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&b[0]).unwrap();
        assert_eq!(v["input"][0], "tài liệu: cái giếng ở đâu");
    }

    #[test]
    fn gui_dimensions_bat_tat_duoc() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g).with_send_dimensions(false);
        e.embed(EmbedRole::Document, &["a"]).unwrap();
        let b = e.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&b[0]).unwrap();
        assert!(v.get("dimensions").is_none(), "{v}");
    }

    #[test]
    fn encoding_format_luon_la_float() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g);
        e.embed(EmbedRole::Document, &["a"]).unwrap();
        let b = e.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&b[0]).unwrap();
        assert_eq!(v["encoding_format"], "float");
    }

    #[test]
    fn loi_422_goi_y_dung_cho_can_sua() {
        let g = Gia::moi(vec![(
            422,
            r#"{"error":{"message":"unknown field `dimensions`"}}"#.to_owned(),
        )]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 4, g);
        let err = e.embed(EmbedRole::Document, &["a"]).unwrap_err();
        assert!(err.to_string().contains("send_dimensions"), "{err}");
    }

    #[test]
    fn thieu_vector_thi_bao_chu_khong_tra_ve_thieu() {
        let g = Gia::moi(vec![(200, than(&[(0, vec![1.0])]))]);
        let e = HttpEmbedder::new("http://x/v1", "k", "m", 1, g);
        let err = e.embed(EmbedRole::Document, &["a", "b"]).unwrap_err();
        assert!(err.to_string().contains("nhận 1 vector"), "{err}");
    }

    #[test]
    fn debug_khong_in_khoa() {
        let g = Gia::moi(vec![]);
        let e = HttpEmbedder::new("http://x/v1", "sk-or-v1-KHOATHAT", "m", 4, g);
        let s = format!("{e:?}");
        assert!(!s.contains("KHOATHAT"), "{s}");
    }
}
