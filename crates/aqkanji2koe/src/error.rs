use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// jpreprocess の初期化に失敗した
    #[error("初期化エラー: {0}")]
    Init(String),

    /// テキスト処理中にエラーが発生した
    #[error("テキスト処理エラー: {0}")]
    Processing(String),
}

pub type Result<T> = std::result::Result<T, Error>;
