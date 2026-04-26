#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use neri_player_desktop::commands::{
    player_cmd, library_cmd, search_cmd, lyrics_cmd, settings_cmd, auth_cmd, recommend_cmd, sync_cmd, download_cmd,
};
use neri_player_desktop::state::AppState;
use neri_player_desktop::auth;
use tauri::{Manager, Emitter};
use std::time::Duration;

fn main() {
    // 强制 WebView2 (Chromium) 启用 GPU 硬件加速
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--enable-gpu --enable-gpu-rasterization --enable-zero-copy --enable-features=CanvasOopRasterization");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            let handle = app.handle().clone();

            // 恢复持久化的登录 Cookie
            {
                let state = handle.state::<AppState>();
                let saved_auth = auth::cookies::load_auth(&handle);
                auth::cookies::inject_all(&state.cookie_jar, &saved_auth);
                *state.auth.lock() = saved_auth;
            }

            // 后台定时器：每 200ms 推送播放位置
            std::thread::spawn(move || {
                let mut last_ended = false;

                loop {
                    std::thread::sleep(Duration::from_millis(200));

                    let state = handle.state::<AppState>();

                    // ── Phase 1: 快速快照（锁持有 <1μs） ──
                    let snapshot = {
                        let player = state.player.lock();
                        if player.current_path.is_none() {
                            last_ended = false;
                            continue;
                        }
                        (
                            player.is_playing,
                            player.position_ms(),
                            player.duration_ms,
                            player.shared_audio_level.clone(),
                        )
                    }; // ← 锁在此释放

                    let (snap_playing, snap_pos, snap_dur, shared_level) = snapshot;

                    // ── Phase 2: 发射事件（无锁） ──
                    if snap_playing || snap_pos > 0 {
                        let _ = handle.emit("player:position", serde_json::json!({
                            "positionMs": snap_pos,
                            "durationMs": snap_dur,
                            "isPlaying": snap_playing,
                        }));
                    }

                    if snap_playing {
                        if let Ok(audio) = shared_level.lock() {
                            let _ = handle.emit("player:audio-level", serde_json::json!({
                                "level": audio.level,
                                "beat": audio.beat_impulse,
                            }));
                        }
                    }

                    // ── Phase 3: 慢检测 — 重新获取锁（is_finished 内部有 200ms recv） ──
                    {
                        let mut player = state.player.lock();
                        let finished = player.is_finished() && player.is_playing && player.position_ms() > 500;
                        if finished && !last_ended {
                            last_ended = true;
                            player.mark_ended();
                            drop(player);
                            let _ = handle.emit("player:track-ended", ());
                        } else if !finished {
                            last_ended = false;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            player_cmd::play_file,
            player_cmd::play_url,
            player_cmd::pause,
            player_cmd::resume,
            player_cmd::toggle_play_pause,
            player_cmd::set_volume,
            player_cmd::seek,
            player_cmd::stop,
            player_cmd::set_speed,
            player_cmd::set_loudness_gain,
            player_cmd::set_equalizer,
            player_cmd::reset_audio_effects,
            player_cmd::get_player_state,
            player_cmd::next_track,
            player_cmd::prev_track,
            player_cmd::set_queue,
            player_cmd::toggle_shuffle,
            player_cmd::cycle_repeat,
            library_cmd::scan_music_directory,
            library_cmd::list_playlists,
            library_cmd::create_playlist,
            library_cmd::delete_playlist,
            library_cmd::rename_playlist,
            library_cmd::get_playlist_tracks,
            library_cmd::add_to_playlist,
            library_cmd::remove_from_playlist,
            library_cmd::list_favorite_playlists,
            search_cmd::search,
            lyrics_cmd::parse_lrc_content,
            lyrics_cmd::load_lyrics_file,
            lyrics_cmd::fetch_lyrics,
            settings_cmd::get_app_data_dir,
            settings_cmd::get_netease_song_url,
            settings_cmd::get_bili_audio_url,
            settings_cmd::get_youtube_audio_url,
            settings_cmd::save_file_bytes,
            auth_cmd::login_netease,
            auth_cmd::login_bilibili,
            auth_cmd::login_youtube,
            auth_cmd::login_with_cookies,
            auth_cmd::check_auth_status,
            auth_cmd::logout,
            recommend_cmd::get_recommended_playlists,
            recommend_cmd::get_recommended_songs,
            recommend_cmd::get_user_playlists,
            recommend_cmd::get_user_account,
            recommend_cmd::get_home_feed,
            recommend_cmd::get_high_quality_playlists,
            recommend_cmd::get_high_quality_tags,
            recommend_cmd::like_song,
            recommend_cmd::get_liked_song_ids,
            recommend_cmd::get_album_detail,
            recommend_cmd::get_user_stared_albums,
            recommend_cmd::get_bili_fav_folder_info,
            recommend_cmd::get_bili_favorite_items,
            recommend_cmd::validate_auth,
            recommend_cmd::get_netease_playlist_detail,
            recommend_cmd::get_youtube_playlist_detail,
            sync_cmd::get_github_sync_config,
            sync_cmd::validate_github_token,
            sync_cmd::create_github_repo,
            sync_cmd::use_existing_github_repo,
            sync_cmd::configure_github_sync,
            sync_cmd::sync_github,
            sync_cmd::disconnect_github_sync,
            sync_cmd::update_github_sync_settings,
            sync_cmd::update_webdav_sync_settings,
            sync_cmd::clear_app_cache,
            sync_cmd::export_playlists,
            sync_cmd::import_playlists,
            sync_cmd::get_webdav_sync_config,
            sync_cmd::configure_webdav_sync,
            sync_cmd::sync_webdav,
            sync_cmd::disconnect_webdav_sync,
            download_cmd::download_track,
            download_cmd::list_downloads,
            download_cmd::delete_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
