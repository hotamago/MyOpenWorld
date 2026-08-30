//! Vị từ của `run_until`.
//!
//! Ngôn ngữ nhỏ nhất đủ dùng:
//!
//! ```text
//! event.kind == 'crime.committed' && event.actor == @villager.aren
//! entity(@aren).need.hunger < 100 || tick >= 5000
//! ```
//!
//! Cố ý **không** dùng một ngôn ngữ script thật. Vị từ chạy sau mỗi tick, và
//! một ngôn ngữ đầy đủ ở vị trí đó sẽ mở đường cho kịch bản chứa logic — lúc
//! đó kịch bản không còn kiểm tra thế giới nữa mà bắt đầu mô phỏng lại nó, và
//! hai bản mô phỏng sẽ trôi khỏi nhau.
//!
//! Ngữ pháp:
//!
//! ```text
//! or    := and ( '||' and )*
//! and   := cmp ( '&&' cmp )*
//! cmp   := term op term
//! op    := '==' | '!=' | '<' | '<=' | '>' | '>='
//! term  := path | number | string | alias
//! ```

use std::collections::BTreeMap;

/// Lỗi phân tích vị từ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "vị từ không hợp lệ: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Một toán hạng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// Đường dẫn tới một giá trị trong ngữ cảnh, ví dụ `event.kind`.
    Path(String),
    /// Hằng số nguyên.
    Int(i64),
    /// Hằng chuỗi.
    Text(String),
    /// Alias, ví dụ `@villager.aren`. Runner thay bằng id đã ràng buộc.
    Alias(String),
}

/// Phép so sánh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Bằng.
    Eq,
    /// Khác.
    Ne,
    /// Nhỏ hơn.
    Lt,
    /// Nhỏ hơn hoặc bằng.
    Le,
    /// Lớn hơn.
    Gt,
    /// Lớn hơn hoặc bằng.
    Ge,
}

/// Cây vị từ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// So sánh hai toán hạng.
    Cmp(Term, Op, Term),
    /// Và.
    And(Box<Predicate>, Box<Predicate>),
    /// Hoặc.
    Or(Box<Predicate>, Box<Predicate>),
}

/// Giá trị lúc đánh giá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    /// Số nguyên.
    Int(i64),
    /// Chuỗi.
    Text(String),
    /// Không có. So sánh với `Missing` luôn cho `false`, **kể cả `!=`**.
    ///
    /// Đây là lựa chọn có chủ đích và đáng nghi ngờ, nên nói rõ: nếu `!=` với
    /// một trường không tồn tại trả `true`, thì một lỗi chính tả trong tên
    /// trường sẽ làm `run_until` dừng ngay tick đầu tiên và kịch bản xanh mà
    /// chẳng chứng minh gì.
    Missing,
}

/// Ngữ cảnh đánh giá: đường dẫn → giá trị.
pub type Bindings = BTreeMap<String, Val>;

impl Predicate {
    /// Phân tích một vị từ.
    pub fn parse(s: &str) -> Result<Predicate, ParseError> {
        let tokens = tokenize(s)?;
        let mut p = Parser { t: tokens, i: 0 };
        let ra = p.parse_or()?;
        if p.i < p.t.len() {
            return Err(ParseError(format!("thừa ký tự từ `{}`", p.t[p.i])));
        }
        Ok(ra)
    }

    /// Đánh giá. `ctx` cung cấp giá trị cho mọi [`Term::Path`], `aliases` cho
    /// mọi [`Term::Alias`].
    pub fn eval(&self, ctx: &Bindings, aliases: &BTreeMap<String, u64>) -> bool {
        match self {
            Predicate::And(a, b) => a.eval(ctx, aliases) && b.eval(ctx, aliases),
            Predicate::Or(a, b) => a.eval(ctx, aliases) || b.eval(ctx, aliases),
            Predicate::Cmp(l, op, r) => {
                let lv = resolve(l, ctx, aliases);
                let rv = resolve(r, ctx, aliases);
                if lv == Val::Missing || rv == Val::Missing {
                    return false;
                }
                match (lv, rv) {
                    (Val::Int(a), Val::Int(b)) => cmp_int(a, *op, b),
                    (Val::Text(a), Val::Text(b)) => match op {
                        Op::Eq => a == b,
                        Op::Ne => a != b,
                        // Thứ tự chuỗi có định nghĩa, nhưng dùng nó trong vị từ
                        // gần như luôn là lỗi soạn thảo, nên trả `false`.
                        _ => false,
                    },
                    _ => false,
                }
            }
        }
    }

    /// Mọi đường dẫn mà vị từ này cần. Runner dùng để chuẩn bị ngữ cảnh.
    pub fn paths(&self) -> Vec<String> {
        let mut ra = Vec::new();
        self.thu_thap(&mut ra);
        ra.sort();
        ra.dedup();
        ra
    }

    fn thu_thap(&self, out: &mut Vec<String>) {
        match self {
            Predicate::And(a, b) | Predicate::Or(a, b) => {
                a.thu_thap(out);
                b.thu_thap(out);
            }
            Predicate::Cmp(l, _, r) => {
                for t in [l, r] {
                    if let Term::Path(p) = t {
                        out.push(p.clone());
                    }
                }
            }
        }
    }

    /// Mọi alias mà vị từ này nhắc tới.
    pub fn aliases(&self) -> Vec<String> {
        let mut ra = Vec::new();
        self.thu_thap_alias(&mut ra);
        ra.sort();
        ra.dedup();
        ra
    }

    fn thu_thap_alias(&self, out: &mut Vec<String>) {
        match self {
            Predicate::And(a, b) | Predicate::Or(a, b) => {
                a.thu_thap_alias(out);
                b.thu_thap_alias(out);
            }
            Predicate::Cmp(l, _, r) => {
                for t in [l, r] {
                    if let Term::Alias(a) = t {
                        out.push(a.clone());
                    }
                }
            }
        }
    }
}

fn cmp_int(a: i64, op: Op, b: i64) -> bool {
    match op {
        Op::Eq => a == b,
        Op::Ne => a != b,
        Op::Lt => a < b,
        Op::Le => a <= b,
        Op::Gt => a > b,
        Op::Ge => a >= b,
    }
}

fn resolve(t: &Term, ctx: &Bindings, aliases: &BTreeMap<String, u64>) -> Val {
    match t {
        Term::Int(v) => Val::Int(*v),
        Term::Text(v) => Val::Text(v.clone()),
        Term::Path(p) => ctx.get(p).cloned().unwrap_or(Val::Missing),
        Term::Alias(a) => aliases
            .get(a)
            .map_or(Val::Missing, |id| Val::Int(*id as i64)),
    }
}

// ── Tách token ───────────────────────────────────────────────────────────────

fn tokenize(s: &str) -> Result<Vec<String>, ParseError> {
    let mut ra = Vec::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '\'' || c == '"' {
            let dong = c;
            let mut buf = String::from(dong);
            i += 1;
            while i < b.len() && b[i] != dong {
                buf.push(b[i]);
                i += 1;
            }
            if i >= b.len() {
                return Err(ParseError("chuỗi không đóng".to_owned()));
            }
            buf.push(dong);
            i += 1;
            ra.push(buf);
        } else if "&|=!<>".contains(c) {
            let mut buf = String::from(c);
            if i + 1 < b.len() && "&|=".contains(b[i + 1]) {
                buf.push(b[i + 1]);
                i += 1;
            }
            i += 1;
            ra.push(buf);
        } else if c == '(' || c == ')' {
            ra.push(c.to_string());
            i += 1;
        } else {
            let mut buf = String::new();
            while i < b.len() && !b[i].is_whitespace() && !"&|=!<>()".contains(b[i]) {
                buf.push(b[i]);
                i += 1;
            }
            ra.push(buf);
        }
    }
    Ok(ra)
}

struct Parser {
    t: Vec<String>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.t.get(self.i).map(String::as_str)
    }

    fn parse_or(&mut self) -> Result<Predicate, ParseError> {
        let mut l = self.parse_and()?;
        while self.peek() == Some("||") {
            self.i += 1;
            let r = self.parse_and()?;
            l = Predicate::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_and(&mut self) -> Result<Predicate, ParseError> {
        let mut l = self.parse_cmp()?;
        while self.peek() == Some("&&") {
            self.i += 1;
            let r = self.parse_cmp()?;
            l = Predicate::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_cmp(&mut self) -> Result<Predicate, ParseError> {
        if self.peek() == Some("(") {
            self.i += 1;
            let inner = self.parse_or()?;
            if self.peek() != Some(")") {
                return Err(ParseError("thiếu `)`".to_owned()));
            }
            self.i += 1;
            return Ok(inner);
        }
        let l = self.parse_term()?;
        let op = match self.peek() {
            Some("==") => Op::Eq,
            Some("!=") => Op::Ne,
            Some("<") => Op::Lt,
            Some("<=") => Op::Le,
            Some(">") => Op::Gt,
            Some(">=") => Op::Ge,
            Some(k) => return Err(ParseError(format!("mong đợi toán tử so sánh, gặp `{k}`"))),
            None => return Err(ParseError("thiếu toán tử so sánh".to_owned())),
        };
        self.i += 1;
        let r = self.parse_term()?;
        Ok(Predicate::Cmp(l, op, r))
    }

    fn parse_term(&mut self) -> Result<Term, ParseError> {
        let Some(t) = self.peek().map(str::to_owned) else {
            return Err(ParseError("thiếu toán hạng".to_owned()));
        };
        self.i += 1;
        if t.starts_with('\'') || t.starts_with('"') {
            return Ok(Term::Text(t[1..t.len() - 1].to_owned()));
        }
        if t.starts_with('@') {
            return Ok(Term::Alias(t));
        }
        if let Ok(v) = t.parse::<i64>() {
            return Ok(Term::Int(v));
        }
        if t.is_empty() {
            return Err(ParseError("toán hạng rỗng".to_owned()));
        }
        Ok(Term::Path(t))
    }
}
