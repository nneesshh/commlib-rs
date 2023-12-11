use arc_swap::ArcSwap;

use crate::conf::Conf;

lazy_static::lazy_static! {
    pub static ref G_CONF: ArcSwap<Conf> = ArcSwap::from_pointee(Conf::new());
}
