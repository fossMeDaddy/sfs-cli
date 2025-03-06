use phf::phf_map;

pub const RANDOM_FILENAME_LEN: usize = 24;

pub const UNTITLED_TAG_PREFX: &str = "untitled_";

pub const ROOT_ACCESS_TOKEN_TAG: &str = "login";

pub const LOCAL_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub const FILE_STREAM_READ_BUF_SIZE: u32 = 256 * 1024;

pub const HEADER_UPLOAD_METADATA: &str = "upload-metadata";

pub const UNKNOWN_FILE_EXT: &str = "bin";
pub const UNKNOWN_MIME_TYPE: &str = "application/octet-stream";
pub const ZIPFILE_MIME_TYPE: &str = "application/zip";
pub static MIME_TYPES: phf::Map<&'static str, &'static str> = phf_map! {
    "aac" => "audio/aac",
    "abw" => "application/x-abiword",
    "arc" => "application/x-freearc",
    "avif" => "image/avif",
    "avi" => "video/x-msvideo",
    "azw" => "application/vnd.amazon.ebook",
    "bin" => UNKNOWN_MIME_TYPE,
    "bmp" => "image/bmp",
    "bz" => "application/x-bzip",
    "bz2" => "application/x-bzip2",
    "cda" => "application/x-cdf",
    "csh" => "application/x-csh",
    "css" => "text/css",
    "csv" => "text/csv",
    "doc" => "application/msword",
    "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "eot" => "application/vnd.ms-fontobject",
    "epub" => "application/epub+zip",
    "gz" => "application/gzip",
    "gif" => "image/gif",
    "htm" => "text/html",
    "html" => "text/html",
    "ico" => "image/vnd.microsoft.icon",
    "ics" => "text/calendar",
    "jar" => "application/java-archive",
    "jpeg" => "image/jpeg",
    "jpg" => "image/jpeg",
    "js" => "text/javascript",
    "json" => "application/json",
    "jsonld" => "application/ld+json",
    "mid" => "audio/midi",
    "midi" => "audio/midi",
    "mjs" => "text/javascript",
    "mp3" => "audio/mpeg",
    "mp4" => "video/mp4",
    "mpeg" => "video/mpeg",
    "mpkg" => "application/vnd.apple.installer+xml",
    "odp" => "application/vnd.oasis.opendocument.presentation",
    "ods" => "application/vnd.oasis.opendocument.spreadsheet",
    "odt" => "application/vnd.oasis.opendocument.text",
    "oga" => "audio/ogg",
    "ogv" => "video/ogg",
    "ogx" => "application/ogg",
    "opus" => "audio/opus",
    "otf" => "font/otf",
    "png" => "image/png",
    "pdf" => "application/pdf",
    "php" => "application/x-httpd-php",
    "ppt" => "application/vnd.ms-powerpoint",
    "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "rar" => "application/vnd.rar",
    "rtf" => "application/rtf",
    "sh" => "application/x-sh",
    "svg" => "image/svg+xml",
    "swf" => "application/x-shockwave-flash",
    "tar" => "application/x-tar",
    "tif" => "image/tiff",
    "tiff" => "image/tiff",
    "ts" => "video/mp2t",
    "ttf" => "font/ttf",
    "txt" => "text/plain",
    "vsd" => "application/vnd.visio",
    "wav" => "audio/wav",
    "weba" => "audio/webm",
    "webm" => "video/webm",
    "webp" => "image/webp",
    "woff" => "font/woff",
    "woff2" => "font/woff2",
    "xhtml" => "application/xhtml+xml",
    "xls" => "application/vnd.ms-excel",
    "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "xml" => "application/xml",
    "xul" => "application/vnd.mozilla.xul+xml",
    "zip" => ZIPFILE_MIME_TYPE,
    "3gp" => "video/3gpp",
    "3g2" => "video/3gpp2",
    "7z" => "application/x-7z-compressed",
};

pub const API_ERR_ALREADY_EXISTS: &str = "ERR_ALREADY_EXISTS";
