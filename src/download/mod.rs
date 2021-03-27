use egs_api::api::types::download_manifest::FileChunkPart;

#[derive(Default, Debug, Clone)]
pub(crate) struct DownloadedFile {
    pub(crate) asset: String,
    pub(crate) release: String,
    pub(crate) name: String,
    pub(crate) chunks: Vec<FileChunkPart>,
    pub(crate) finished_chunks: Vec<FileChunkPart>,
}
