/// use thread local unsafe cell -- mut
#[macro_export]
macro_rules! with_tls_mut {
    ($t:path, $c:ident, $body:block) => {
        $t.with(|v| {
            paste::paste! {
                let $c = unsafe { &mut *v.get() };
                $body
            }
        })
    };
}

/// use thread local unsafe cell
#[macro_export]
macro_rules! with_tls {
    ($t:path, $c:ident, $body:block) => {
        $t.with(|v| {
            paste::paste! {
                let $c = unsafe { &*v.get() };
                $body
            }
        })
    };
}
