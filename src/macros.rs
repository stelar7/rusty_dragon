#[macro_export]
macro_rules! parse_single {
    ($type:ident, $data:ident) => {
        $type::<_, VerboseError<_>>($data).unwrap().1
    };

    ($type:ident, $data:expr) => {
        $type::<_, VerboseError<_>>($data).unwrap().1
    };
}

#[macro_export]
macro_rules! parse_tuple {
    ($type:expr, $data:ident) => {
        tuple::<_, _, VerboseError<_>, _>($type)($data).unwrap().1
    };

    ($type:expr, $data:expr) => {
        tuple::<_, _, VerboseError<_>, _>($type)($data).unwrap().1
    };
}
