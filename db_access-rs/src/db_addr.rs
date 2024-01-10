//!
//! Database addr
//!

///
#[derive(clap::Args, Debug)]
pub struct MySqlAddr {
    #[arg(short = 'u', long, value_name = "USER", verbatim_doc_comment)]
    pub user: String,

    #[arg(short = 'p', long, value_name = "PASSWORD", verbatim_doc_comment)]
    pub password: String,

    #[arg(short = 'h', long, value_name = "HOST", verbatim_doc_comment)]
    pub host: String,

    #[arg(
        short = 'P',
        long,
        default_value = "3306",
        value_name = "PORT",
        verbatim_doc_comment
    )]
    pub port: u16,

    #[arg(short = 'D', long, value_name = "DBNAME", verbatim_doc_comment)]
    pub dbname: String,
}