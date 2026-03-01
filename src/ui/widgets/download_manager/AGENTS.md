# DOWNLOAD_MANAGER — Asset Download Engine

## OVERVIEW

Manages chunked downloads of Epic/Fab assets with pause/resume/cancel, hash validation, and extraction. Attached to main window, not a visible tab.

## STRUCTURE

```
download_manager/
├── mod.rs           # EpicDownloadManager — CompositeTemplate widget, queue management
├── asset.rs         # Asset trait — download orchestration: manifest → chunks → validate → extract
├── download_item.rs # EpicDownloadItem — single download row widget with progress
├── epic_file.rs     # EpicFile — file-level download tracking and chunk assembly
└── docker.rs        # Docker image download for engine builds
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Change download flow | `asset.rs` | `add_asset_download()` → `start_download_asset()` → `download_asset_file()` |
| Modify chunk logic | `epic_file.rs` | Handles chunk splitting, reassembly, SHA1 validation |
| Change download UI | `download_item.rs` | Progress bars, pause/resume buttons, status labels |
| Add post-download action | `asset.rs` | `PostDownloadAction` enum (CreateProject, AddToProject) |
| Modify Docker downloads | `docker.rs` | Registry auth + layer download for UE engine builds |
| Change queue management | `mod.rs` | `add_download()`, `process_downloads()`, tick signal |

## CONVENTIONS

- **Trait-based extension**: `Asset` trait implemented on `EpicDownloadManager` — keeps download logic separate from widget code
- **Three thread pools**: `download_pool` (HTTP), `file_pool` (disk I/O), `thumbnail_pool` (images)
- **ThreadMessages enum**: `Pause`, `Resume`, `Cancel` sent to running download threads via channel
- **Chunk-based downloads**: Files split into chunks, downloaded in parallel, reassembled with hash validation
- **Tick signal**: `EpicDownloadManager` emits `"tick"` signal for UI progress updates

## ANTI-PATTERNS

- **asset.rs:462** — `//TODO: This has the potential to loop forever` — re-download without retry limit. **Critical bug risk.**
- **asset.rs:278** — download manifest not cached, re-fetched every session
- **asset.rs:1045** — hash validation failure has no retry mechanism
- **mod.rs:481,544** — `//TODO: Report downloaded size` — progress reporting incomplete
- `DownloadedFile` struct has no serialization — **downloads don't survive app restart**

## FAB DOWNLOAD INTEGRATION

FAB assets use a different download flow via `egs-api`:
1. `fab_asset_manifest(artifact_id, namespace, asset_id, platform)` → `Vec<DownloadInfo>`
2. `fab_download_manifest(download_info, distribution_point_url)` → `DownloadManifest`
3. `DownloadManifest` contains the same chunk structure as Epic assets — reuse existing chunk download logic
