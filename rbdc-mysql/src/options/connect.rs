use crate::connection::MySqlConnection;
use crate::options::MySqlConnectOptions;
use futures_core::future::BoxFuture;
use rbdc::db::{ConnectOptions, Connection};
use rbdc::Error;
use std::any::Any;
use std::str::FromStr;

impl ConnectOptions for MySqlConnectOptions {
    fn connect(&self) -> BoxFuture<Result<Box<dyn Connection>, Error>> {
        Box::pin(async move {
            let conn = MySqlConnection::establish(self).await?;

            // After the connection is established, we initialize by configuring a few
            // connection parameters

            // https://mariadb.com/kb/en/sql-mode/

            // PIPES_AS_CONCAT - Allows using the pipe character (ASCII 124) as string concatenation operator.
            //                   This means that "A" || "B" can be used in place of CONCAT("A", "B").

            // NO_ENGINE_SUBSTITUTION - If not set, if the available storage engine specified by a CREATE TABLE is
            //                          not available, a warning is given and the default storage
            //                          engine is used instead.

            // NO_ZERO_DATE - Don't allow '0000-00-00'. This is invalid in Rust.

            // NO_ZERO_IN_DATE - Don't allow 'YYYY-00-00'. This is invalid in Rust.

            // --

            // Setting the time zone allows us to assume that the output
            // from a TIMESTAMP field is UTC

            // --

            // https://mathiasbynens.be/notes/mysql-utf8mb4

            // let mut options = String::new();
            // // options.push_str(r#"SET sql_mode=(SELECT CONCAT(@@sql_mode, ',PIPES_AS_CONCAT,NO_ENGINE_SUBSTITUTION')),"#);
            // // options.push_str(r#"time_zone='+00:00',"#);
            // options.push_str(&format!(
            //     r#"SET NAMES {} COLLATE {};"#,
            //     conn.stream.charset.as_str(),
            //     conn.stream.collation.as_str()
            // ));
            //
            // conn.execute(&*options).await?;

            let r: Box<dyn Connection> = Box::new(conn);
            Ok(r)
        })
    }

    fn set_uri(&mut self, uri: &str) -> Result<(), Error> {
        *self = MySqlConnectOptions::from_str(uri).map_err(|e| Error::from(e.to_string()))?;
        Ok(())
    }

    fn uppercase_self(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}
