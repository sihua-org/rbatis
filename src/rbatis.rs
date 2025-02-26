use crate::executor::{RBatisConnExecutor, RBatisTxExecutor};
use crate::plugin::intercept::SqlIntercept;
use crate::plugin::log::{LogPlugin, RbatisLogPlugin};
use crate::snowflake::new_snowflake_id;
use crate::Error;
use dark_std::sync::SyncVec;
use once_cell::sync::OnceCell;
use rbdc::db::{Connection, Driver};
use rbdc::pool::{ManagerPorxy, Pool};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

/// rbatis engine
#[derive(Clone)]
pub struct Rbatis {
    // the connection pool,use OnceCell init this
    pub pool: Arc<OnceCell<Pool>>,
    // sql intercept vec chain
    pub sql_intercepts: Arc<SyncVec<Box<dyn SqlIntercept>>>,
    // log plugin
    pub log_plugin: Arc<Box<dyn LogPlugin>>,
}

impl Debug for Rbatis {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rbatis")
            .field("pool", &self.pool)
            .field("sql_intercepts", &self.sql_intercepts.len())
            .finish()
    }
}

impl Default for Rbatis {
    fn default() -> Rbatis {
        Rbatis::new()
    }
}

///Rbatis Options
pub struct RbatisOption {
    /// sql intercept vec chain
    pub sql_intercepts: SyncVec<Box<dyn SqlIntercept>>,
    /// log plugin
    pub log_plugin: Box<dyn LogPlugin>,
}

impl Default for RbatisOption {
    fn default() -> Self {
        Self {
            sql_intercepts: SyncVec::new(),
            log_plugin: Box::new(RbatisLogPlugin::default()) as Box<dyn LogPlugin>,
        }
    }
}

impl Rbatis {
    ///create an Rbatis
    pub fn new() -> Self {
        return Self::new_with_opt(RbatisOption::default());
    }

    ///new Rbatis from Option
    pub fn new_with_opt(option: RbatisOption) -> Self {
        return Self {
            pool: Arc::new(OnceCell::new()),
            sql_intercepts: Arc::new(option.sql_intercepts),
            log_plugin: Arc::new(option.log_plugin),
        };
    }

    /// init() and try_acquire a connection.
    /// use init() replace this method
    #[deprecated]
    pub async fn link<Driver: rbdc::db::Driver + 'static>(
        &self,
        driver: Driver,
        url: &str,
    ) -> Result<(), Error> {
        self.init(driver, url)?;
        self.try_acquire().await?;
        Ok(())
    }

    /// init pool
    pub fn init<Driver: rbdc::db::Driver + 'static>(
        &self,
        driver: Driver,
        url: &str,
    ) -> Result<(), Error> {
        if url.is_empty() {
            return Err(Error::from("[rbatis] link url is empty!"));
        }
        let mut option = driver.default_option();
        option.set_uri(url)?;
        let pool = Pool::new_box(Box::new(driver), option)?;
        self.pool
            .set(pool)
            .map_err(|_e| Error::from("pool set fail!"))?;
        return Ok(());
    }

    /// init pool
    pub async fn init_builder<Driver: rbdc::db::Driver + 'static>(
        &self,
        builder: rbdc::deadpool::managed::PoolBuilder<
            ManagerPorxy,
            rbdc::deadpool::managed::Object<ManagerPorxy>,
        >,
        driver: Driver,
        url: &str,
    ) -> Result<(), Error> {
        if url.is_empty() {
            return Err(Error::from("[rbatis] link url is empty!"));
        }
        let mut option = driver.default_option();
        option.set_uri(url)?;
        let pool = Pool::new_builder(builder, Box::new(driver), option)?;
        self.pool
            .set(pool)
            .map_err(|_e| Error::from("pool set fail!"))?;
        return Ok(());
    }

    /// init pool by DBPoolOptions
    /// for example:
    ///
    pub fn init_opt<
        Driver: rbdc::db::Driver + 'static,
        ConnectOptions: rbdc::db::ConnectOptions,
    >(
        &self,
        driver: Driver,
        options: ConnectOptions,
    ) -> Result<(), Error> {
        let pool = Pool::new(driver, options)?;
        self.pool
            .set(pool)
            .map_err(|_e| Error::from("pool set fail!"))?;
        return Ok(());
    }

    /// set_log_plugin
    pub fn set_log_plugin(&mut self, arg: impl LogPlugin + 'static) {
        self.log_plugin = Arc::new(Box::new(arg));
    }

    /// set_sql_intercepts for many
    pub fn set_sql_intercepts(&mut self, arg: Vec<Box<dyn SqlIntercept>>) {
        self.sql_intercepts = Arc::new(SyncVec::from(arg));
    }

    /// get conn pool
    ///
    /// can set option for example:
    /// ```rust
    /// use rbatis::Rbatis;
    /// let rb = Rbatis::new();
    /// //rb.init(rbdc_sqlite::driver::SqliteDriver {},"sqlite://target/sqlite.db");
    /// //rb.get_pool().unwrap().resize(10);
    /// ```
    pub fn get_pool(&self) -> Result<&Pool, Error> {
        let p = self
            .pool
            .get()
            .ok_or_else(|| Error::from("[rbatis] rbatis pool not inited!"))?;
        return Ok(p);
    }

    /// get driver type
    pub fn driver_type(&self) -> Result<&str, Error> {
        let pool = self.get_pool()?;
        Ok(pool.driver_type())
    }

    /// get an DataBase Connection used for the next step
    pub async fn acquire(&self) -> Result<RBatisConnExecutor, Error> {
        let pool = self.get_pool()?;
        let conn = pool.get().await?;
        return Ok(RBatisConnExecutor {
            conn: Box::new(conn),
            rb: self.clone(),
        });
    }

    /// try get an DataBase Connection used for the next step
    pub async fn try_acquire(&self) -> Result<RBatisConnExecutor, Error> {
        let pool = self.get_pool()?;
        let mut default = pool.inner.timeouts().clone();
        default.wait = Some(Duration::ZERO);
        let conn = pool.timeout_get(&default).await?;
        return Ok(RBatisConnExecutor {
            conn: Box::new(conn),
            rb: self.clone(),
        });
    }

    /// get an DataBase Connection,and call begin method,used for the next step
    pub async fn acquire_begin(&self) -> Result<RBatisTxExecutor, Error> {
        let pool = self.get_pool()?;
        let mut conn = pool.get().await?;
        conn.exec("begin", vec![]).await?;
        return Ok(RBatisTxExecutor {
            tx_id: new_snowflake_id(),
            conn: Box::new(conn),
            rb: self.clone(),
            done: false,
        });
    }

    /// try get an DataBase Connection,and call begin method,used for the next step
    pub async fn try_acquire_begin(&self) -> Result<RBatisTxExecutor, Error> {
        let mut conn = self.try_acquire().await?;
        conn.exec("begin", vec![]).await?;
        return Ok(RBatisTxExecutor {
            tx_id: new_snowflake_id(),
            conn: conn.conn,
            rb: self.clone(),
            done: false,
        });
    }

    /// is debug mode
    pub fn is_debug_mode(&self) -> bool {
        crate::decode::is_debug_mode()
    }

    /// get driver
    pub fn driver(&self) -> Option<&dyn Driver> {
        if let Ok(v) = self.get_pool() {
            Some(&*v.manager.driver)
        } else {
            None
        }
    }
}
