pub trait WrappedError:
    std::fmt::Debug + std::fmt::Display + std::error::Error
{
}

#[macro_export]
macro_rules! simple_error {
    (
        $vis:vis
        $err_ty:ident
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Default)]
        $vis struct $err_ty();

        impl std::fmt::Display for $err_ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, stringify!($err_ty))
            }
        }

        impl std::error::Error for $err_ty {}

        impl $crate::error::WrappedError for $err_ty {}
    };
}

#[macro_export]
macro_rules! custom_error {
    (
        $vis:vis
        $err_ty:ident
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $err_ty(String);

        impl $err_ty {
            pub fn new(msg: impl Into<String>) -> Self {
                Self(msg.into())
            }
        }

        impl std::fmt::Display for $err_ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!(stringify!($err_ty), "({})"), self.0)
            }
        }

        impl std::error::Error for $err_ty {}

        impl $crate::error::WrappedError for $err_ty {}
    };
}

#[macro_export]
macro_rules! enumerated_error {
    (
        $vis:vis
        $err_ty:ident,
        $($variant:ident($from_ty:ty)),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis enum $err_ty {
            $(
                $variant($from_ty),
            )*
        }

        impl std::fmt::Display for $err_ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                    Self::$variant(e) => write!(f, concat!(stringify!($err_ty), "({})"), e),
                    )*
                }
            }
        }

        $(
        impl From<$from_ty> for $err_ty where
            $from_ty: $crate::error::WrappedError{
            fn from(e: $from_ty) -> Self {
                Self::$variant(e)
            }
        }
        )*

        impl std::error::Error for $err_ty {}

        impl $crate::error::WrappedError for $err_ty {}
    };
}
