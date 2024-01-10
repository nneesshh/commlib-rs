//!
//! Database accessor
//!

use std::cell::UnsafeCell;
use std::ops::Index;
use std::rc::Rc;

use chrono::{DateTime, Datelike, Timelike, Utc};
use mysql::{from_row, prelude::Queryable, Pool, PooledConn, Result, Row, Statement, Value};

use crate::utils::mysql_util;

///
pub const DEFAULT_PARAMS_CAPACITY: usize = 256;

const EXEC_RETRY_MAX: i32 = 2;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(3000);
const DEFAULT_ROWS_CAPACITY: usize = 256;
const DEFAULT_COLUMNS_CAPACITY: usize = 64;

///
pub struct SqlRows {
    pub rows: Vec<SqlRow>,
    pub columns: Vec<String>,
}

impl SqlRows {
    ///
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

///
pub struct SqlRow(Row);

impl SqlRow {
    /// idx auto increase 1
    #[inline(always)]
    pub fn get(&self, idx: &mut usize) -> &Value {
        let val = self.0.index(*idx);
        (*idx) += 1;
        val
    }

    /// by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_by_name(&self, name: &str) -> &Value {
        self.0.index(name)
    }

    /// str
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_str(&self, idx: &mut usize) -> Option<&str> {
        let val = self.get(idx);
        mysql_util::value_to_str(val)
    }

    /// str by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_str_by_name(&self, name: &str) -> Option<&str> {
        let val = self.get_by_name(name);
        mysql_util::value_to_str(val)
    }

    /// blob
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_blob(&self, idx: &mut usize) -> Option<&Vec<u8>> {
        let val = self.get(idx);
        mysql_util::value_to_blob(val)
    }

    /// blob by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_blob_by_name(&self, name: &str) -> Option<&Vec<u8>> {
        let val = self.get_by_name(name);
        mysql_util::value_to_blob(val)
    }

    /// int64
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_int64(&self, idx: &mut usize) -> Option<i64> {
        let val = self.get(idx);
        mysql_util::value_to_int64(val)
    }

    /// int64 by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_int64_by_name(&self, name: &str) -> Option<i64> {
        let val = self.get_by_name(name);
        mysql_util::value_to_int64(val)
    }

    /// uint64
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_uint64(&self, idx: &mut usize) -> Option<u64> {
        let val = self.get(idx);
        mysql_util::value_to_uint64(val)
    }

    /// uint64 by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_uint64_by_name(&self, name: &str) -> Option<u64> {
        let val = self.get_by_name(name);
        mysql_util::value_to_uint64(val)
    }

    /// float
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_float(&self, idx: &mut usize) -> Option<f32> {
        let val = self.get(idx);
        mysql_util::value_to_float(val)
    }

    /// float by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_float_by_name(&self, name: &str) -> Option<f32> {
        let val = self.get_by_name(name);
        mysql_util::value_to_float(val)
    }

    /// double
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_double(&self, idx: &mut usize) -> Option<f64> {
        let val = self.get(idx);
        mysql_util::value_to_double(val)
    }

    /// double by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_double_by_name(&self, name: &str) -> Option<f64> {
        let val = self.get_by_name(name);
        mysql_util::value_to_double(val)
    }

    /// timestamp -- Return (secs, micro_seconds)
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_timestamp(&self, idx: &mut usize) -> Option<(i64, u32)> {
        let val = self.get(idx);
        mysql_util::value_to_timestamp(val)
    }

    /// timestamp by name -- Return (secs, micro_seconds)
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_timestamp_by_name(&self, name: &str) -> Option<(i64, u32)> {
        let val = self.get_by_name(name);
        mysql_util::value_to_timestamp(val)
    }

    /// date
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_date(&self, idx: &mut usize) -> Option<(u16, u8, u8, u8, u8, u8, u32)> {
        let val = self.get(idx);
        mysql_util::value_to_date(val)
    }

    /// date by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_date_by_name(&self, name: &str) -> Option<(u16, u8, u8, u8, u8, u8, u32)> {
        let val = self.get_by_name(name);
        mysql_util::value_to_date(val)
    }

    /// time
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_time(&self, idx: &mut usize) -> Option<(bool, u32, u8, u8, u8, u32)> {
        let val = self.get(idx);
        mysql_util::value_to_time(val)
    }

    /// time by name
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_time_by_name(&self, name: &str) -> Option<(bool, u32, u8, u8, u8, u32)> {
        let val = self.get_by_name(name);
        mysql_util::value_to_time(val)
    }
}

///
pub struct SqlPreparedParams {
    params: Vec<Value>,
}

impl SqlPreparedParams {
    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            params: Vec::with_capacity(DEFAULT_PARAMS_CAPACITY),
        }
    }

    ///
    #[inline(always)]
    pub fn to_inner(self) -> Vec<Value> {
        self.params
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_null(&mut self) {
        self.params.push(Value::NULL);
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_string<S>(&mut self, s: S)
    where
        S: AsRef<str>,
    {
        let v = s.as_ref().as_bytes().to_vec();
        self.params.push(Value::Bytes(v));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_blob(&mut self, blob: Vec<u8>) {
        self.params.push(Value::Bytes(blob));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_int64(&mut self, n: i64) {
        self.params.push(Value::Int(n));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_uint64(&mut self, n: u64) {
        self.params.push(Value::UInt(n));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_float(&mut self, f: f32) {
        self.params.push(Value::Float(f));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_double(&mut self, f: f64) {
        self.params.push(Value::Double(f));
    }

    ///
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_timestamp(&mut self, secs: i64, nsecs: u32) {
        let dt: DateTime<Utc> =
            DateTime::<Utc>::from_timestamp(secs, nsecs).expect("invalid timestamp");

        let year = dt.year() as u16;
        let month = dt.month() as u8;
        let day = dt.day() as u8;
        let hour = dt.hour() as u8;
        let minutes = dt.minute() as u8;
        let seconds = dt.second() as u8;

        assert_eq!(dt.nanosecond(), nsecs);
        if nsecs > 0 {
            let micro_seconds = nsecs / 1000_u32;
            self.add_date(year, month, day, hour, minutes, seconds, micro_seconds);
        } else {
            self.add_date(year, month, day, hour, minutes, seconds, 0);
        }
    }

    /// year, month, day, hour, minutes, seconds, micro seconds
    #[inline(always)]
    pub fn add_date(
        &mut self,
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minutes: u8,
        seconds: u8,
        micro_seconds: u32,
    ) {
        self.params.push(Value::Date(
            year,
            month,
            day,
            hour,
            minutes,
            seconds,
            micro_seconds,
        ));
    }

    /// is negative, days, hours, minutes, seconds, micro seconds
    #[allow(dead_code)]
    #[inline(always)]
    pub fn add_time(
        &mut self,
        is_negative: bool,
        days: u32,
        hours: u8,
        minutes: u8,
        seconds: u8,
        micro_seconds: u32,
    ) {
        self.params.push(Value::Time(
            is_negative,
            days,
            hours,
            minutes,
            seconds,
            micro_seconds,
        ));
    }
}

///
pub struct SqlPrepared {
    pub pstmt_opt: Option<Rc<UnsafeCell<Statement>>>,
}

impl SqlPrepared {
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self { pstmt_opt: None }
    }

    ///
    #[inline(always)]
    pub fn init<S>(&mut self, conn: &mut PooledConn, sql: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        let prep_ret = conn.prep(sql);
        match prep_ret {
            Ok(stmt) => {
                //
                self.pstmt_opt = Some(Rc::new(UnsafeCell::new(stmt)));
                Ok(())
            }
            Err(err) => {
                //
                Err(err)
            }
        }
    }

    ///
    #[inline(always)]
    pub fn clear(&mut self) {
        self.pstmt_opt = None;
    }
}

impl Clone for SqlPrepared {
    fn clone(&self) -> Self {
        let pstmt_opt = self.pstmt_opt.clone();
        Self { pstmt_opt }
    }
}

struct SqlStmt {
    pub conn_opt: Option<PooledConn>,
    pre_stmt_table: hashbrown::HashMap<String, SqlPrepared>,
}

impl SqlStmt {
    ///
    #[inline(always)]
    pub fn new(conn: PooledConn) -> Self {
        Self {
            conn_opt: Some(conn),
            pre_stmt_table: hashbrown::HashMap::with_capacity(1024),
        }
    }

    ///
    #[inline(always)]
    #[allow(dead_code)]
    pub fn prepare<S>(&mut self, sql: &S) -> Result<()>
    where
        S: AsRef<str>,
    {
        let pre_stmt_table = &mut self.pre_stmt_table;
        let conn = self.conn_opt.as_mut().unwrap();
        sql_stmt_prepare(pre_stmt_table, conn, sql)
    }

    ///
    pub fn re_prepare(&mut self, mut conn: PooledConn) -> Result<()> {
        self.conn_opt = None;
        for (_sql, prepared) in &mut self.pre_stmt_table {
            prepared.clear();
        }
        for (sql, prepared) in &mut self.pre_stmt_table {
            prepared.init(&mut conn, sql)?;
        }
        self.conn_opt = Some(conn);
        Ok(())
    }

    ///
    #[inline(always)]
    pub fn get_prepared<S>(&mut self, sql: &S) -> Result<SqlPrepared>
    where
        S: AsRef<str>,
    {
        let pre_stmt_table = &mut self.pre_stmt_table;
        let conn = self.conn_opt.as_mut().unwrap();
        sql_stmt_get_prepared(pre_stmt_table, conn, sql)
    }
}

#[inline(always)]
fn sql_stmt_prepare<S>(
    pre_stmt_table: &mut hashbrown::HashMap<String, SqlPrepared>,
    conn: &mut PooledConn,
    sql: &S,
) -> Result<()>
where
    S: AsRef<str>,
{
    let mut prepared = SqlPrepared::new();
    match prepared.init(conn, sql) {
        Ok(_) => {
            //
            pre_stmt_table.insert(sql.as_ref().to_string(), prepared);
            Ok(())
        }
        Err(err) => {
            //
            Err(err)
        }
    }
}

#[inline(always)]
fn sql_stmt_get_prepared<S>(
    pre_stmt_table: &mut hashbrown::HashMap<String, SqlPrepared>,
    conn: &mut PooledConn,
    sql: &S,
) -> Result<SqlPrepared>
where
    S: AsRef<str>,
{
    let prepared_opt = pre_stmt_table.get_mut(sql.as_ref());
    match prepared_opt {
        Some(prepared) => {
            //
            return Ok(prepared.clone());
        }
        None => {
            // try auto prepare
            sql_stmt_prepare(pre_stmt_table, conn, sql)?;

            // prepare success, get again
            sql_stmt_get_prepared(pre_stmt_table, conn, sql)
        }
    }
}

///
pub struct MySqlAccess {
    url: String,
    pool_opt: Option<Pool>,
    sql_stmt_opt: Option<SqlStmt>,
}

impl MySqlAccess {
    ///
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool_opt: None,
            sql_stmt_opt: None,
        }
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        let pool_ret = Pool::new(self.url.as_str());
        match pool_ret {
            Ok(pool) => {
                //
                let conn_ret = pool.get_conn();
                match conn_ret {
                    Ok(conn) => {
                        //
                        let sql_stmt = SqlStmt::new(conn);

                        //
                        self.sql_stmt_opt = Some(sql_stmt);
                        self.pool_opt = Some(pool);
                        Ok(())
                    }
                    Err(err) => {
                        //
                        log::error!(
                            "open mysql({}) get connection from pool failed!!! err: {}!!!",
                            self.url,
                            err
                        );
                        Err(err)
                    }
                }
            }
            Err(err) => {
                //
                log::error!("open mysql({}) failed!!! err: {}!!!", self.url, err);
                Err(err)
            }
        }
    }

    ///
    pub fn reopen(&mut self) -> Result<()> {
        //
        let pool = self.pool_opt.as_mut().unwrap();
        let sql_stmt = self.sql_stmt_opt.as_mut().unwrap();

        let conn_ret = pool.get_conn();
        match conn_ret {
            Ok(conn) => {
                // re-prepare
                sql_stmt.re_prepare(conn)?;

                //
                Ok(())
            }
            Err(err) => {
                //
                log::error!(
                    "reopen mysql({}) get connection from pool failed!!! err: {}!!!",
                    self.url,
                    err
                );
                Err(err)
            }
        }
    }

    ///
    #[allow(dead_code)]
    pub fn exec_query<S>(&mut self, sql: S) -> Result<SqlRows>
    where
        S: AsRef<str>,
    {
        log::debug!("[exec_query] sql={}", sql.as_ref());

        let mut cnt = 0;
        let mut err_opt = None;
        while cnt < EXEC_RETRY_MAX {
            cnt = cnt + 1;

            let ret = self.do_exec_query(&sql);
            if ret.is_ok() {
                return ret;
            } else {
                err_opt = ret.err();
            }

            // retry, for unexpeced link broken
            log::info!("[exec_query] retry {} after {:?} ...", cnt, RETRY_DELAY);
            std::thread::sleep(RETRY_DELAY);
            let _ = self.reopen();
        }

        // error
        let err = err_opt.unwrap();
        log::error!("[exec_query] failed!!! error: {}", err);
        Err(err)
    }

    ///
    #[allow(dead_code)]
    pub fn exec_update<S>(&mut self, sql: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        log::debug!("[exec_update] sql={}", sql.as_ref());

        let mut cnt = 0;
        let mut err_opt = None;
        while cnt < EXEC_RETRY_MAX {
            cnt = cnt + 1;

            let ret = self.do_exec_update(&sql);
            if ret.is_ok() {
                return ret;
            } else {
                err_opt = ret.err();
            }

            // retry, for unexpeced link broken
            log::info!("[exec_update] retry {} after {:?} ...", cnt, RETRY_DELAY);
            std::thread::sleep(RETRY_DELAY);
            let _ = self.reopen();
        }

        // error
        let err = err_opt.unwrap();
        log::error!("[exec_update] failed!!! error: {}", err);
        Err(err)
    }

    //
    #[allow(dead_code)]
    pub fn exec_prepared_query<F, S>(&mut self, sql: S, params_fn: F) -> Result<SqlRows>
    where
        S: AsRef<str>,
        F: Fn() -> Option<SqlPreparedParams>,
    {
        log::debug!("[exec_prepared_query] sql={}", sql.as_ref());

        let mut cnt = 0;
        let mut err_opt = None;
        while cnt < EXEC_RETRY_MAX {
            cnt = cnt + 1;

            let params_opt = params_fn();
            let param_vec = if let Some(params) = params_opt {
                params.to_inner()
            } else {
                Vec::with_capacity(0)
            };
            let rs = self.do_exec_prepared_query(&sql, param_vec);
            if rs.is_ok() {
                return rs;
            } else {
                err_opt = rs.err();
            }

            // retry, for unexpeced link broken
            log::error!(
                "[exec_prepared_query] retry {} after {:?} ...",
                cnt,
                RETRY_DELAY
            );
            std::thread::sleep(RETRY_DELAY);
            let _ = self.reopen();
        }

        // error
        let err = err_opt.unwrap();
        log::error!("[exec_prepared_query] failed!!! error: {}", err);
        Err(err)
    }

    ///
    #[allow(dead_code)]
    pub fn exec_prepared_update<F, S>(&mut self, sql: S, params_fn: F) -> Result<()>
    where
        S: AsRef<str>,
        F: Fn() -> Option<SqlPreparedParams>,
    {
        log::debug!("[exec_prepared_update] sql={}", sql.as_ref());

        let mut cnt = 0;
        let mut err_opt = None;
        while cnt < EXEC_RETRY_MAX {
            cnt = cnt + 1;

            let params_opt = params_fn();
            let param_vec = if let Some(params) = params_opt {
                params.to_inner()
            } else {
                Vec::with_capacity(0)
            };
            let rs = self.do_exec_prepared_update(&sql, param_vec);
            if rs.is_ok() {
                return rs;
            } else {
                err_opt = rs.err();
            }

            // retry, for unexpeced link broken
            log::error!(
                "[exec_prepared_update] retry {} after {:?} ...",
                cnt,
                RETRY_DELAY
            );
            std::thread::sleep(RETRY_DELAY);
            let _ = self.reopen();
        }

        // error
        let err = err_opt.unwrap();
        log::error!("[exec_prepared_update] failed!!! error: {}", err);
        Err(err)
    }

    #[inline(always)]
    fn do_exec_query<S>(&mut self, sql: &S) -> Result<SqlRows>
    where
        S: AsRef<str>,
    {
        log::info!(
            "\r\n================================\r\n[do_exec_query] sql={}",
            sql.as_ref()
        );

        let pool = self.pool_opt.as_mut().unwrap();
        let mut conn = pool.get_conn()?;

        //
        let mut rows = Vec::with_capacity(DEFAULT_ROWS_CAPACITY);
        let mut columns = Vec::with_capacity(DEFAULT_COLUMNS_CAPACITY);

        //
        let mut result = conn.query_iter(sql)?;
        let mut sets = 0;
        while let Some(rs) = result.iter() {
            sets += 1;

            // collect columns
            for column in rs.columns().as_ref() {
                let column_name = column.org_name_str();
                columns.push(column_name.into_owned());
            }

            log::info!(
                "[do_exec_query] Result set columns: {}/{:?}",
                sets,
                rs.columns()
            );
            log::info!(
                "[do_exec_query] Result set meta: {}, {:?}, {} {}",
                rs.affected_rows(),
                rs.last_insert_id(),
                rs.warnings(),
                rs.info_str(),
            );

            for row in rs {
                rows.push(SqlRow(from_row(row?)));
            }
        }

        Ok(SqlRows { rows, columns })
    }

    #[inline(always)]
    fn do_exec_update<S>(&mut self, sql: &S) -> Result<()>
    where
        S: AsRef<str>,
    {
        log::info!(
            "\r\n================================\r\n[do_exec_update] sql={}",
            sql.as_ref()
        );

        let pool = self.pool_opt.as_mut().unwrap();
        let mut conn = pool.get_conn()?;
        conn.query_drop(sql)
    }

    #[inline(always)]
    fn do_exec_prepared_query<S>(&mut self, sql: &S, param_vec: Vec<Value>) -> Result<SqlRows>
    where
        S: AsRef<str>,
    {
        log::info!(
            "\r\n================================\r\n[do_exec_prepared_query] sql={}",
            sql.as_ref()
        );

        let sql_stmt = self.sql_stmt_opt.as_mut().unwrap();
        let mut prepared = sql_stmt.get_prepared(sql)?;
        let stmt = unsafe { &*prepared.pstmt_opt.as_mut().unwrap().get() };

        //
        let mut rows = Vec::with_capacity(DEFAULT_ROWS_CAPACITY);
        let mut columns = Vec::with_capacity(DEFAULT_COLUMNS_CAPACITY);

        //
        let mut sets = 0;

        let conn = sql_stmt.conn_opt.as_mut().unwrap();
        let mut result = conn.exec_iter(stmt.clone(), param_vec)?;

        while let Some(rs) = result.iter() {
            sets += 1;

            // collect columns
            for column in rs.columns().as_ref() {
                let column_name = column.org_name_str();
                columns.push(column_name.into_owned());
            }

            // log::info!(
            //     "\r\n================================\r\n[do_exec_prepared_query] Result set columns: {:?}",
            //     rs.columns()
            // );
            log::info!(
                "\r\n================================\r\n[do_exec_prepared_query]({}) Result set meta: {}, {:?}, {} {}",
                sets,
                rs.affected_rows(),
                rs.last_insert_id(),
                rs.warnings(),
                rs.info_str(),
            );

            for row in rs {
                rows.push(SqlRow(from_row(row?)));
            }
        }

        Ok(SqlRows { rows, columns })
    }

    #[inline(always)]
    fn do_exec_prepared_update<S>(&mut self, sql: &S, param_vec: Vec<Value>) -> Result<()>
    where
        S: AsRef<str>,
    {
        log::info!(
            "\r\n================================\r\n[do_exec_prepared_update] sql={}",
            sql.as_ref()
        );

        let sql_stmt = self.sql_stmt_opt.as_mut().unwrap();
        let mut prepared = sql_stmt.get_prepared(sql)?;
        let stmt = unsafe { &*prepared.pstmt_opt.as_mut().unwrap().get() };

        //
        let conn = sql_stmt.conn_opt.as_mut().unwrap();
        conn.exec_drop(stmt.clone(), param_vec)
    }
}

#[cfg(test)]
mod test {

    use crate::{db_access::SqlPreparedParams, sqls};

    use super::{MySqlAccess, MySqlAddr};

    #[test]
    fn msyql_query() {
        let db_addr = MySqlAddr {
            user: "root".to_owned(),
            password: "123456".to_owned(),
            host: "localhost".to_owned(),
            port: 3306_u16,
            dbname: "test_gpaas".to_owned(),
        };
        let url = std::format!(
            "mysql://{}:{}@{}:{}/{}",
            db_addr.user,
            db_addr.password,
            db_addr.host,
            db_addr.port,
            db_addr.dbname,
        );
        let mut db = MySqlAccess::new(url.as_str());
        match db.open() {
            Ok(_) => {
                //
            }
            Err(err) => {
                //
                std::panic!("{}", err);
            }
        };

        let ret = db.exec_prepared_query(sqls::SQL_INSERT_ZONE, || {
            //
            let mut params = SqlPreparedParams::new();
            params.add_uint64(5001);

            params.add_uint64(4);
            params.add_uint64(20001);
            params.add_uint64(8888);
            params.add_string("mysql://db_url");
            params.add_string("localhost");
            params.add_uint64(443);
            params.add_string("http://web_url");

            params.add_null(); // null blob?
            params.add_null(); // null timestamp?

            params.add_uint64(4);
            params.add_uint64(20001);
            params.add_uint64(8888);
            params.add_string("mysql://db_url");
            params.add_string("localhost");
            params.add_uint64(443);
            params.add_string("http://web_url");

            params.add_blob("blablabla".as_bytes().to_vec());
            params.add_timestamp(1703830842, 0);

            params
        });
        assert!(ret.is_ok());
    }
}
