# LOGGED_IN — Main Application UI (Post-Merge Architecture)

## OVERVIEW

Post-authentication UI container. `EpicLoggedInBox` is a 3-column horizontal layout:
- **Left**: `EpicSidebar` — page navigation + category filters + download/favorite toggles
- **Center**: `GtkStack(page_stack)` — two pages: `"unreal"` (collapsible sections) and `"games"`
- **Right**: `EpicAssetDetails` — shared detail panel, hidden by default

## STRUCTURE

```
logged_in/
├── mod.rs           # EpicLoggedInBox — 3-column layout, page_stack orchestration (244 lines)
├── refresh.rs       # Refresh trait — shared asset-fetching interface
├── log_line.rs      # Single log line widget
├── logs.rs          # Log viewer panel
├── plugins.rs       # Plugin listing (minimal)
├── library/         # Epic Store assets (embedded in "unreal" page as collapsible section)
│   ├── mod.rs         # EpicLibraryBox — grid + filter + sort + search
│   ├── asset.rs       # EpicAsset — single grid item widget
│   ├── asset_detail.rs # EpicAssetDetails — detail panel (shared at logged_in level)
│   ├── image_stack.rs # Image carousel for asset previews
│   ├── sidebar/       # EpicSidebar — page nav + category tree filter
│   │   ├── mod.rs       # EpicSidebar — navigation buttons + category stack + filters
│   │   ├── button.rs    # EpicSidebarButton — clickable nav item
│   │   ├── categories.rs # EpicSidebarCategories — category tree for filtering
│   │   └── category.rs  # Individual category item
│   └── actions/       # Download, add-to-project, create-project dialogs
├── engines/         # Unreal Engine installations (collapsible section in "unreal" page)
│   ├── mod.rs         # EpicEnginesBox
│   ├── engines_side.rs # Engine sidebar
│   ├── epic_download.rs # Epic CDN engine download
│   └── docker_download.rs # Docker-based engine build
├── projects/        # UE Projects (collapsible section in "unreal" page)
│   ├── mod.rs         # EpicProjectsBox — project scanner
│   └── project_detail.rs # Project detail panel
└── games/           # Games page (separate page_stack page)
    └── mod.rs         # EpicGamesBox — grid of games/DLC
```

## ARCHITECTURE (Post-Merge)

### Page Stack Layout
```
logged_in.ui:
  GtkBox(horizontal)
  ├── EpicSidebar                    ← always visible, drives page switching
  ├── GtkStack(page_stack)
  │   ├── "unreal" page (GtkBox vertical)
  │   │   ├── GtkExpander "ENGINE VERSIONS"
  │   │   │   └── EpicEnginesBox
  │   │   ├── GtkExpander "MY PROJECTS"
  │   │   │   └── EpicProjectsBox
  │   │   └── GtkExpander "ASSET LIBRARY" (vexpand)
  │   │       └── EpicLibraryBox
  │   └── "games" page
  │       └── EpicGamesBox
  └── EpicAssetDetails(hidden)       ← shared detail panel
```

### Sidebar Navigation
- `EpicSidebar` has 3 top-level buttons: `all_category`, `unreal_category`, `games_category`
- Each button maps to a `EpicSidebarCategories` in a stack (with category sub-filtering)
- `set_page_stack()` connects sidebar to `page_stack` for page switching
- `switch_main_page(page)` calls `stack.set_visible_child_name(page)`

### Wiring Flow (set_window)
```
EpicLoggedInBox::set_window()
  → details.set_window()
  → library.set_details(&details)        ← library gets shared detail panel ref
  → library.set_window()
  → library.set_sidebar(&sidebar)        ← library gets sidebar for category filtering
  → sidebar.set_page_stack(&page_stack)  ← sidebar gets page_stack for navigation
  → engines.set_window()
  → projects.set_window()
  → games.set_window()
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new page to stack | `logged_in.ui` + `mod.rs` | Add GtkStackPage + TemplateChild |
| Add sidebar nav entry | `sidebar/mod.rs` + `sidebar.ui` | New EpicSidebarButton + EpicSidebarCategories |
| Modify asset grid | `library/mod.rs` | FilterListModel + SortListModel + GridView |
| Change detail panel | `library/asset_detail.rs` | CompositeTemplate — shared across pages |
| Add download action | `library/actions/` | Each action is a separate CompositeTemplate |
| Modify page switching | `sidebar/mod.rs` | `switch_main_page()` + button click handlers |

## CONVENTIONS

- **Collapsible sections**: "unreal" page uses `GtkExpander` with `section` CSS class
- **Dual thread pools**: Library has `asset_load_pool` (15 threads) + `image_load_pool` (15 threads)
- **Refresh trait**: `refresh.rs` defines shared `Refresh` trait for async data fetching
- **Asset caching**: JSON cached to `{cache_dir}/epic_asset.json`
- **Sorting**: `sorter()` static method returns `CustomSorter` — name, released, updated (asc/desc)
- **Filtering**: `CustomFilter` on search text + category path with `&`, `|`, `!` operators
- **Page-level detail panel**: `EpicAssetDetails` is at `EpicLoggedInBox` level, not per-page

## ANTI-PATTERNS

- `library/sidebar/mod.rs:33` — TODO about reusing window's expand_button
- `library/image_stack.rs:288` — TODO about having one sender/receiver per widget
- `library/mod.rs:778` — dead code: `//return; // TODO: REMOVE AFTER FAB`
- **Deprecated widgets in .ui files**: GtkInfoBar (→ AdwBanner), GtkComboBoxText (→ GtkDropDown)
