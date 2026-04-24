// ProtoBuf 消息定义 — 与 Android 端 SyncDataModels.kt 的 @ProtoNumber 对齐
// 用于省流模式 (backup.bin: ProtoBuf + GZIP + Base64)

/// SyncData 根容器
#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncData {
    #[prost(string, tag = "1")]
    pub version: String,
    #[prost(string, tag = "2")]
    pub device_id: String,
    #[prost(string, tag = "3")]
    pub device_name: String,
    #[prost(int64, tag = "4")]
    pub last_modified: i64,
    #[prost(message, repeated, tag = "5")]
    pub playlists: Vec<ProtoSyncPlaylist>,
    #[prost(message, repeated, tag = "6")]
    pub favorite_playlists: Vec<ProtoSyncFavoritePlaylist>,
    #[prost(message, repeated, tag = "7")]
    pub recent_plays: Vec<ProtoSyncRecentPlay>,
    #[prost(message, repeated, tag = "8")]
    pub sync_log: Vec<ProtoSyncLogEntry>,
    #[prost(message, repeated, tag = "9")]
    pub recent_play_deletions: Vec<ProtoSyncRecentPlayDeletion>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncPlaylist {
    #[prost(int64, tag = "1")]
    pub id: i64,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(message, repeated, tag = "3")]
    pub songs: Vec<ProtoSyncSong>,
    #[prost(int64, tag = "4")]
    pub created_at: i64,
    #[prost(int64, tag = "5")]
    pub modified_at: i64,
    #[prost(bool, tag = "6")]
    pub is_deleted: bool,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncSong {
    #[prost(int64, tag = "1")]
    pub id: i64,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub artist: String,
    #[prost(string, tag = "4")]
    pub album: String,
    #[prost(int64, tag = "5")]
    pub album_id: i64,
    #[prost(int64, tag = "6")]
    pub duration_ms: i64,
    #[prost(string, optional, tag = "7")]
    pub cover_url: Option<String>,
    #[prost(string, optional, tag = "8")]
    pub media_uri: Option<String>,
    #[prost(int64, tag = "9")]
    pub added_at: i64,
    #[prost(string, optional, tag = "10")]
    pub matched_lyric: Option<String>,
    #[prost(string, optional, tag = "11")]
    pub matched_translated_lyric: Option<String>,
    #[prost(string, optional, tag = "12")]
    pub matched_lyric_source: Option<String>,
    #[prost(string, optional, tag = "13")]
    pub matched_song_id: Option<String>,
    #[prost(int64, tag = "14")]
    pub user_lyric_offset_ms: i64,
    #[prost(string, optional, tag = "15")]
    pub custom_cover_url: Option<String>,
    #[prost(string, optional, tag = "16")]
    pub custom_name: Option<String>,
    #[prost(string, optional, tag = "17")]
    pub custom_artist: Option<String>,
    #[prost(string, optional, tag = "18")]
    pub original_name: Option<String>,
    #[prost(string, optional, tag = "19")]
    pub original_artist: Option<String>,
    #[prost(string, optional, tag = "20")]
    pub original_cover_url: Option<String>,
    #[prost(string, optional, tag = "21")]
    pub original_lyric: Option<String>,
    #[prost(string, optional, tag = "22")]
    pub original_translated_lyric: Option<String>,
    #[prost(string, optional, tag = "23")]
    pub channel_id: Option<String>,
    #[prost(string, optional, tag = "24")]
    pub audio_id: Option<String>,
    #[prost(string, optional, tag = "25")]
    pub sub_audio_id: Option<String>,
    #[prost(string, optional, tag = "26")]
    pub playlist_context_id: Option<String>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncRecentPlay {
    #[prost(int64, tag = "1")]
    pub song_id: i64,
    #[prost(message, optional, tag = "2")]
    pub song: Option<ProtoSyncSong>,
    #[prost(int64, tag = "3")]
    pub played_at: i64,
    #[prost(string, tag = "4")]
    pub device_id: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncRecentPlayDeletion {
    #[prost(int64, tag = "1")]
    pub song_id: i64,
    #[prost(string, tag = "2")]
    pub album: String,
    #[prost(string, optional, tag = "3")]
    pub media_uri: Option<String>,
    #[prost(int64, tag = "4")]
    pub deleted_at: i64,
    #[prost(string, tag = "5")]
    pub device_id: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncFavoritePlaylist {
    #[prost(int64, tag = "1")]
    pub id: i64,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, optional, tag = "3")]
    pub cover_url: Option<String>,
    #[prost(int32, tag = "4")]
    pub track_count: i32,
    #[prost(string, tag = "5")]
    pub source: String,
    #[prost(message, repeated, tag = "6")]
    pub songs: Vec<ProtoSyncSong>,
    #[prost(int64, tag = "7")]
    pub added_time: i64,
    #[prost(int64, tag = "8")]
    pub modified_at: i64,
    #[prost(bool, tag = "9")]
    pub is_deleted: bool,
    #[prost(int64, tag = "10")]
    pub sort_order: i64,
    #[prost(string, optional, tag = "11")]
    pub browse_id: Option<String>,
    #[prost(string, optional, tag = "12")]
    pub playlist_id: Option<String>,
    #[prost(string, optional, tag = "13")]
    pub subtitle: Option<String>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtoSyncLogEntry {
    #[prost(int64, tag = "1")]
    pub timestamp: i64,
    #[prost(string, tag = "2")]
    pub device_id: String,
    #[prost(int32, tag = "3")]
    pub action: i32,
    #[prost(int64, optional, tag = "4")]
    pub playlist_id: Option<i64>,
    #[prost(int64, optional, tag = "5")]
    pub song_id: Option<i64>,
    #[prost(string, optional, tag = "6")]
    pub details: Option<String>,
}
