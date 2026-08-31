//! Hiện thực thứ hai của [`VectorIndex`]: Qdrant (`plan.md §P3.4`, `PC-20`).
//!
//! ## Vì sao dùng REST chứ không dùng `qdrant-client`
//!
//! `qdrant-client` là gRPC và bất đồng bộ; kéo nó vào đây có nghĩa là kéo cả
//! `tonic` và một runtime cho một trait đồng bộ. REST của Qdrant làm được đúng
//! những gì trait này cần — upsert, search có filter, delete, count — và một
//! client HTTP đồng bộ nhỏ hơn cả một bậc độ lớn về phụ thuộc.
//!
//! Đây là `§P1` nguyên tắc 2 áp dụng ngược chiều thường lệ: đứng trên vai người
//! khổng lồ không có nghĩa là lấy cái thư viện to nhất, mà là **không tự viết
//! lại cái khó** — ở đây cái khó là chỉ mục vector, không phải cách nói HTTP.
//!
//! ## Lọc dòng dõi phải nằm trong truy vấn, không nằm sau nó
//!
//! Chỗ dễ sai nhất của backend này. Cách tự nhiên là hỏi Qdrant lấy `limit`
//! điểm gần nhất rồi lọc dòng dõi ở phía Rust. Nó cho ra kết quả **trông đúng**
//! và sai theo hai cách cùng lúc:
//!
//! 1. Trả về ít hơn `limit` một cách khó đoán, vì phần lớn kết quả bị lọc bỏ.
//! 2. Tệ hơn: nếu 100 điểm gần nhất đều thuộc nhánh khác, kết quả **rỗng** —
//!    trong khi ký ức hợp lệ vẫn tồn tại, chỉ là hơi xa hơn. Nhân vật mất trí
//!    nhớ một cách chọn lọc, và không có gì báo lỗi.
//!
//! Nên bộ lọc `namespace` và dòng dõi được gửi **cùng** truy vấn, và Qdrant
//! chọn `limit` điểm gần nhất *trong số hợp lệ*. Điều kiện dòng dõi
//! `created_branch = B AND created_tick <= cutoff(B)` trở thành một mệnh đề
//! `should` gồm nhiều `must` — mỗi bậc dòng dõi một mệnh đề.
//!
//! ## Chạy test hợp đồng
//!
//! ```bash
//! ./mow infra up
//! MOW_QDRANT_URL=http://localhost:6333 \
//!   cargo test -p mow-vector --features qdrant -- --ignored
//! ```

use crate::{Hit, MemoryId, MemoryPoint, Query, VectorError, VectorIndex, VectorResult};
use mow_core::{BranchId, Tick};
use serde_json::{json, Value};

fn loi(e: impl std::fmt::Display) -> VectorError {
    VectorError::External(e.to_string())
}

/// Chỉ mục trên Qdrant.
pub struct QdrantIndex {
    base: String,
    collection: String,
    dim: usize,
    agent: ureq::Agent,
}

impl QdrantIndex {
    /// Kết nối và tạo collection nếu chưa có.
    pub fn connect(base_url: &str, collection: &str, dim: usize) -> VectorResult<QdrantIndex> {
        let idx = QdrantIndex {
            base: base_url.trim_end_matches('/').to_owned(),
            collection: collection.to_owned(),
            dim,
            agent: ureq::Agent::new_with_defaults(),
        };
        idx.ensure_collection()?;
        Ok(idx)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/collections/{}{path}", self.base, self.collection)
    }

    fn ensure_collection(&self) -> VectorResult<()> {
        // `Dot` chứ không phải `Cosine`: `crate::quantize` đã chuẩn hóa L2 trước
        // khi lượng tử hóa, nên tích vô hướng **là** cosine. Để Qdrant chuẩn hóa
        // lại lần nữa sẽ cho thứ hạng hơi khác backend nhúng, và "hơi khác" ở
        // đây nghĩa là hai backend trả về hai ký ức khác nhau cho cùng truy vấn.
        let body = json!({
            "vectors": { "size": self.dim, "distance": "Dot" }
        });
        let r = self.agent.put(&self.url("")).send_json(&body);
        match r {
            // 409 nghĩa là collection đã tồn tại — chuyện bình thường ở lần chạy
            // thứ hai, không phải lỗi.
            Ok(_) | Err(ureq::Error::StatusCode(409)) => Ok(()),
            Err(e) => Err(loi(e)),
        }
    }

    /// Điều kiện lọc, gửi **cùng** truy vấn. Xem phần đầu file.
    fn filter(q: &Query) -> Value {
        let ns: Vec<Value> = q
            .namespaces
            .iter()
            .map(|n| json!({ "key": "namespace", "match": { "value": n } }))
            .collect();

        let dong_doi: Vec<Value> = q
            .lineage
            .iter()
            .map(|s| {
                json!({
                    "must": [
                        { "key": "created_branch", "match": { "value": s.branch.get() as i64 } },
                        { "key": "created_tick", "range": { "lte": s.cutoff.0 as i64 } },
                    ]
                })
            })
            .collect();

        json!({
            "must": [
                // Rỗng nghĩa là **không lấy gì**: mặc định an toàn là "không
                // thấy gì" chứ không phải "thấy tất cả" (xem `Query`).
                { "should": ns },
                { "should": dong_doi },
                // Đã quên trên nhánh hiện tại thì không trả về.
                {
                    "must_not": [{
                        "key": "tombstoned_in",
                        "match": { "any": q.lineage.first().map(|s| s.branch.get() as i64) },
                    }]
                },
            ]
        })
    }
}

impl VectorIndex for QdrantIndex {
    fn dimension(&self) -> usize {
        self.dim
    }

    fn upsert(&mut self, point: &MemoryPoint) -> VectorResult<()> {
        if point.vector.len() != self.dim {
            return Err(VectorError::Dimension {
                got: point.vector.len(),
                want: self.dim,
            });
        }
        let body = json!({
            "points": [{
                "id": point.id.0,
                // Qdrant chỉ nhận `f32`. Chuyển ngược từ `i16` là **không mất
                // mát**: `i16` nằm gọn trong 24 bit định trị của `f32`, nên thứ
                // hạng giữ nguyên. Đây là chỗ duy nhất trong toàn hệ thống mà số
                // thực được phép xuất hiện, và nó nằm ngoài đường commit
                // (`§P10.2.1`).
                "vector": point.vector.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
                "payload": {
                    "namespace": point.namespace,
                    "persona_version": point.persona_version,
                    "created_branch": point.created_branch.get() as i64,
                    "created_tick": point.created_tick.0 as i64,
                    "tombstoned_in": Vec::<i64>::new(),
                    "blob": point.payload,
                }
            }]
        });
        self.agent
            .put(&self.url("/points?wait=true"))
            .send_json(&body)
            .map_err(loi)?;
        Ok(())
    }

    fn tombstone(&mut self, id: MemoryId, branch: BranchId) -> VectorResult<()> {
        // Đánh dấu chứ không xóa: nhánh chị em vẫn phải thấy ký ức đó.
        let body = json!({
            "points": [id.0],
            "key": "tombstoned_in",
            "value": branch.get() as i64,
        });
        // Qdrant không có "append vào mảng payload" nguyên tử, nên đọc–sửa–ghi.
        // Chấp nhận được vì tombstone đi qua đường commit của Rust, và ở đó chỉ
        // có một người ghi (`§22.1`).
        let cu: Value = self
            .agent
            .get(&format!("{}/points/{}", self.url(""), id.0))
            .call()
            .map_err(loi)?
            .body_mut()
            .read_json()
            .map_err(loi)?;

        let mut ds: Vec<i64> = cu["result"]["payload"]["tombstoned_in"]
            .as_array()
            .map(|a| a.iter().filter_map(serde_json::Value::as_i64).collect())
            .unwrap_or_default();
        let b = branch.get() as i64;
        if !ds.contains(&b) {
            ds.push(b);
        }

        let _ = body;
        self.agent
            .post(&self.url("/points/payload?wait=true"))
            .send_json(json!({
                "points": [id.0],
                "payload": { "tombstoned_in": ds },
            }))
            .map_err(loi)?;
        Ok(())
    }

    fn search(&self, q: &Query) -> VectorResult<Vec<Hit>> {
        if q.namespaces.is_empty() || q.lineage.is_empty() {
            return Ok(Vec::new());
        }
        let body = json!({
            "vector": q.vector.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
            "filter": Self::filter(q),
            "limit": q.limit,
            "with_payload": true,
            "with_vector": true,
        });
        let ra: Value = self
            .agent
            .post(&self.url("/points/search"))
            .send_json(&body)
            .map_err(loi)?
            .body_mut()
            .read_json()
            .map_err(loi)?;

        let mut hits: Vec<Hit> = Vec::new();
        for r in ra["result"].as_array().into_iter().flatten() {
            let p = &r["payload"];
            hits.push(Hit {
                point: MemoryPoint {
                    id: MemoryId(r["id"].as_u64().unwrap_or(0)),
                    namespace: p["namespace"].as_str().unwrap_or("").to_owned(),
                    persona_version: u32::try_from(p["persona_version"].as_u64().unwrap_or(0))
                        .unwrap_or(0),
                    created_branch: BranchId(p["created_branch"].as_u64().unwrap_or(0)),
                    created_tick: Tick(p["created_tick"].as_u64().unwrap_or(0)),
                    vector: r["vector"]
                        .as_array()
                        .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0) as i16).collect())
                        .unwrap_or_default(),
                    payload: p["blob"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| u8::try_from(v.as_u64().unwrap_or(0)).ok())
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                // Qdrant trả điểm `f32`; nhân lên để về số nguyên như hợp đồng
                // đòi. Thứ hạng giữ nguyên, và **thứ hạng** mới là thứ hợp đồng
                // nói tới — giá trị tuyệt đối của điểm chưa bao giờ so được giữa
                // hai backend.
                score: (r["score"].as_f64().unwrap_or(0.0) * 1_000_000.0) as i64,
            });
        }
        // Phá hòa bằng `id`: hợp đồng đòi kết quả **xác định**, và Qdrant không
        // hứa thứ tự cho hai điểm cùng điểm số.
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.point.id.0.cmp(&b.point.id.0)));
        Ok(hits)
    }

    fn clear(&mut self) -> VectorResult<()> {
        self.agent
            .post(&self.url("/points/delete?wait=true"))
            .send_json(json!({ "filter": {} }))
            .map_err(loi)?;
        Ok(())
    }

    fn len(&self) -> VectorResult<usize> {
        let ra: Value = self
            .agent
            .post(&self.url("/points/count"))
            .send_json(json!({ "exact": true }))
            .map_err(loi)?
            .body_mut()
            .read_json()
            .map_err(loi)?;
        Ok(usize::try_from(ra["result"]["count"].as_u64().unwrap_or(0)).unwrap_or(0))
    }
}
