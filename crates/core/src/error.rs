use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("không đọc/ghi được file: {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("không nhận diện được định dạng nén từ tên file: {0}")]
    UnknownFormat(PathBuf),

    #[error("định dạng {0:?} chưa hỗ trợ thao tác này")]
    UnsupportedOperation(crate::Format),

    #[error("lỗi xử lý file nén: {0}")]
    Archive(String),

    #[error("cần mật khẩu để giải nén file này")]
    PasswordRequired,

    #[error("mật khẩu không đúng")]
    WrongPassword,
}

impl Error {
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    /// Mã lỗi ổn định (không đổi giữa các bản dịch) để lớp giao diện (Desktop/Android)
    /// tự dịch sang ngôn ngữ hiện tại thay vì chỉ hiển thị `Display` (vốn cố định tiếng
    /// Việt). Các lỗi còn lại (Io, Archive) chưa có bản dịch riêng — dùng `Display` làm
    /// phương án dự phòng ở giao diện.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io { .. } => "io",
            Error::UnknownFormat(_) => "unknown_format",
            Error::UnsupportedOperation(_) => "unsupported_operation",
            Error::Archive(_) => "archive",
            Error::PasswordRequired => "password_required",
            Error::WrongPassword => "wrong_password",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_is_stable_for_translatable_errors() {
        assert_eq!(Error::PasswordRequired.kind(), "password_required");
        assert_eq!(Error::WrongPassword.kind(), "wrong_password");
        assert_eq!(
            Error::UnknownFormat("x.unknown".into()).kind(),
            "unknown_format"
        );
        assert_eq!(
            Error::UnsupportedOperation(crate::Format::Rar).kind(),
            "unsupported_operation"
        );
    }
}
