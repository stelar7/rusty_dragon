#[macro_export]
macro_rules! parse_single {
    ($type:ident, $data:ident) => {
        $type::<_, VerboseError<_>>($data).unwrap().1
    };

    ($type:ident, $data:expr) => {
        $type::<_, VerboseError<_>>($data).unwrap().1
    };
}
