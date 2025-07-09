macro_rules! try2 {
    (async move $x:expr) => {{
        let _r: std::result::Result<_, _> = async move { $x }.await;
        _r
    }};

    (async $x:expr) => {{
        let _r: std::result::Result<_, _> = async { $x }.await;
        _r
    }};

    (move $x:expr) => {{
        let _r: std::result::Result<_, _> = (move || $x)();
        _r
    }};

    ($x:expr) => {{
        let _r: std::result::Result<_, _> = (|| $x)();
        _r
    }};
}
