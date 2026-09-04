//! `rusqlite` 0.32.1 — only what `storage/sqlite` uses.
//!
//! `connection.rs` becomes a provided file exposing a small driver interface
//! (spec 1a), so `Connection` itself never reaches TypeScript. `engine.rs`
//! does get transpiled, and it calls `prepare`, `execute`, `execute_batch`,
//! `query_row` and `query_map`, with `Row::get` inside the row closures — those
//! are the signatures the engine has to resolve to type the closure returns.

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Connection;

impl Connection {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Connection> { todo!() }
    pub fn open_in_memory() -> Result<Connection> { todo!() }
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> { todo!() }
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> { todo!() }
    pub fn execute_batch(&self, sql: &str) -> Result<()> { todo!() }
    pub fn query_row<T, P: Params, F: FnOnce(&Row<'_>) -> Result<T>>(&self, sql: &str, params: P, f: F) -> Result<T> { todo!() }
    pub fn last_insert_rowid(&self) -> i64 { todo!() }
    pub fn changes(&self) -> u64 { todo!() }
    pub fn close(self) -> std::result::Result<(), (Connection, Error)> { todo!() }
}

impl Debug for Connection { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct Statement<'conn>;

impl<'conn> Statement<'conn> {
    pub fn execute<P: Params>(&mut self, params: P) -> Result<usize> { todo!() }
    pub fn query<P: Params>(&mut self, params: P) -> Result<Rows<'_>> { todo!() }
    pub fn query_map<T, P: Params, F: FnMut(&Row<'_>) -> Result<T>>(&mut self, params: P, f: F) -> Result<MappedRows<'_, F>> { todo!() }
    pub fn query_row<T, P: Params, F: FnOnce(&Row<'_>) -> Result<T>>(&mut self, params: P, f: F) -> Result<T> { todo!() }
    pub fn column_count(&self) -> usize { todo!() }
    pub fn column_names(&self) -> Vec<&str> { todo!() }
}

pub struct Rows<'stmt>;

impl<'stmt> Rows<'stmt> {
    pub fn next(&mut self) -> Result<Option<&Row<'stmt>>> { todo!() }
}

pub struct MappedRows<'stmt, F>;

impl<'stmt, T, F: FnMut(&Row<'_>) -> Result<T>> Iterator for MappedRows<'stmt, F> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Result<T>> { todo!() }
}

pub struct Row<'stmt>;

impl<'stmt> Row<'stmt> {
    pub fn get<I: RowIndex, T: types::FromSql>(&self, idx: I) -> Result<T> { todo!() }
    pub fn get_ref<I: RowIndex>(&self, idx: I) -> Result<types::ValueRef<'_>> { todo!() }
    pub fn get_unwrap<I: RowIndex, T: types::FromSql>(&self, idx: I) -> T { todo!() }
    pub fn column_count(&self) -> usize { todo!() }
}

pub trait RowIndex {}

impl RowIndex for usize {}
impl<'a> RowIndex for &'a str {}

/// rusqlite is deliberately narrow about what can be a parameter list. There
/// is no generic `&[T]` or `Vec<T>` impl: an earlier version of this file
/// declared both, which accepted containers rusqlite rejects. The real set is
/// the empty array, a slice of trait objects, the unit, tuples, fixed-size
/// arrays of trait objects, and `ParamsFromIter` over an iterator whose items
/// are themselves `ToSql`.
pub trait Params {}

impl Params for [&(dyn types::ToSql + Send + Sync); 0] {}
impl Params for () {}
impl<'a> Params for &'a [&'a dyn types::ToSql] {}
impl<'a> Params for &'a [(&'a str, &'a dyn types::ToSql)] {}
impl<const N: usize> Params for [&dyn types::ToSql; N] {}
impl<T: types::ToSql> Params for (T,) {}
impl<T1: types::ToSql, T2: types::ToSql> Params for (T1, T2) {}
impl<T1: types::ToSql, T2: types::ToSql, T3: types::ToSql> Params for (T1, T2, T3) {}
impl<T1: types::ToSql, T2: types::ToSql, T3: types::ToSql, T4: types::ToSql> Params for (T1, T2, T3, T4) {}
impl<T1: types::ToSql, T2: types::ToSql, T3: types::ToSql, T4: types::ToSql, T5: types::ToSql> Params for (T1, T2, T3, T4, T5) {}
impl<I: IntoIterator> Params for ParamsFromIter<I> where <I as IntoIterator>::Item: types::ToSql {}

pub struct ParamsFromIter<I>;

pub fn params_from_iter<I: IntoIterator>(iter: I) -> ParamsFromIter<I>
where <I as IntoIterator>::Item: types::ToSql { todo!() }

pub mod types {
    pub enum Value {
        Null,
        Integer(i64),
        Real(f64),
        Text(String),
        Blob(Vec<u8>),
    }

    impl Clone for Value { fn clone(&self) -> Value { todo!() } }
    impl Debug for Value { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl PartialEq for Value { fn eq(&self, other: &Value) -> bool { todo!() } }

    pub enum ValueRef<'a> {
        Null,
        Integer(i64),
        Real(f64),
        Text(&'a [u8]),
        Blob(&'a [u8]),
    }

    impl<'a> ValueRef<'a> {
        pub fn as_str(&self) -> Result<&'a str, FromSqlError> { todo!() }
        pub fn as_i64(&self) -> Result<i64, FromSqlError> { todo!() }
        pub fn as_f64(&self) -> Result<f64, FromSqlError> { todo!() }
        pub fn as_blob(&self) -> Result<&'a [u8], FromSqlError> { todo!() }
        pub fn data_type(&self) -> Type { todo!() }
    }

    pub enum Type {
        Null,
        Integer,
        Real,
        Text,
        Blob,
    }

    impl Debug for Type { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl std::fmt::Display for Type { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

    pub trait ToSql {
        fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error>;
    }

    pub enum ToSqlOutput<'a> {
        Borrowed(ValueRef<'a>),
        Owned(Value),
    }

    pub trait FromSql: Sized {
        fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError>;
    }

    pub enum FromSqlError {
        InvalidType,
        OutOfRange(i64),
        Other(Box<dyn std::error::Error + Send + Sync + 'static>),
    }

    impl Debug for FromSqlError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl std::fmt::Display for FromSqlError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl std::error::Error for FromSqlError {}

    impl ToSql for Value { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for String { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for str { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for i64 { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for f64 { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for bool { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl ToSql for Vec<u8> { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl<T: ToSql> ToSql for Option<T> { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }
    impl<T: ToSql + ?Sized> ToSql for &T { fn to_sql(&self) -> Result<ToSqlOutput<'_>, super::Error> { todo!() } }

    impl FromSql for String { fn column_result(value: ValueRef<'_>) -> Result<String, FromSqlError> { todo!() } }
    impl FromSql for i64 { fn column_result(value: ValueRef<'_>) -> Result<i64, FromSqlError> { todo!() } }
    impl FromSql for i32 { fn column_result(value: ValueRef<'_>) -> Result<i32, FromSqlError> { todo!() } }
    impl FromSql for f64 { fn column_result(value: ValueRef<'_>) -> Result<f64, FromSqlError> { todo!() } }
    impl FromSql for bool { fn column_result(value: ValueRef<'_>) -> Result<bool, FromSqlError> { todo!() } }
    impl FromSql for Vec<u8> { fn column_result(value: ValueRef<'_>) -> Result<Vec<u8>, FromSqlError> { todo!() } }
    impl FromSql for Value { fn column_result(value: ValueRef<'_>) -> Result<Value, FromSqlError> { todo!() } }
    impl<T: FromSql> FromSql for Option<T> { fn column_result(value: ValueRef<'_>) -> Result<Option<T>, FromSqlError> { todo!() } }
}

pub enum Error {
    SqliteFailure(ffi::Error, Option<String>),
    QueryReturnedNoRows,
    InvalidColumnIndex(usize),
    InvalidColumnName(String),
    InvalidColumnType(usize, String, types::Type),
    FromSqlConversionFailure(usize, types::Type, Box<dyn std::error::Error + Send + Sync + 'static>),
    IntegralValueOutOfRange(usize, i64),
    Utf8Error(std::str::Utf8Error),
    NulError(std::ffi::NulError),
    InvalidPath(std::path::PathBuf),
    ExecuteReturnedResults,
    InvalidParameterName(String),
    ToSqlConversionFailure(Box<dyn std::error::Error + Send + Sync + 'static>),
    MultipleStatement,
}

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for Error {}
impl PartialEq for Error { fn eq(&self, other: &Error) -> bool { todo!() } }

pub mod ffi {
    pub struct Error;

    impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl std::fmt::Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
}
