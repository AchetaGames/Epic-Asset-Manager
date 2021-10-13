global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('PKGDATADIR', pkgdatadir)
global_conf.set_quoted('PROFILE', profile)
global_conf.set_quoted('VERSION', version + version_suffix)
global_conf.set_quoted('GETTEXT_PACKAGE', gettext_package)
global_conf.set_quoted('LOCALEDIR', localedir)
config = configure_file(
  input: 'config.rs.in',
  output: 'config.rs',
  configuration: global_conf
)
# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.build_root() / 'src' / 'config.rs',
  meson.source_root() / 'src' / 'config.rs',
  check: true
)

rust_sources = files(
    'window.rs',
    'tools/asset_info.rs',
    'tools/or.rs',
    'tools/mod.rs',
    'application.rs',
    'main.rs',
    'models/asset_data.rs',
    'models/engine_data.rs',
    'models/category_data.rs',
    'models/database.rs',
    'models/project_data.rs',
    'models/mod.rs',
    'ui/update.rs',
    'ui/authentication.rs',
    'ui/widgets/preferences/window.rs',
    'ui/widgets/preferences/dir_row.rs',
    'ui/widgets/preferences/mod.rs',
    'ui/widgets/logged_in/mod.rs',
    'ui/widgets/logged_in/library/category.rs',
    'ui/widgets/logged_in/library/asset.rs',
    'ui/widgets/logged_in/engines/mod.rs',
    'ui/widgets/logged_in/engines/engine.rs',
    'ui/widgets/logged_in/projects/mod.rs',
    'ui/widgets/logged_in/projects/project.rs',
    'ui/widgets/logged_in/projects/project_detail.rs',
    'ui/widgets/logged_in/library/asset_detail.rs',
    'ui/widgets/logged_in/library/image_stack.rs',
    'ui/widgets/logged_in/library/download_detail.rs',
    'ui/widgets/logged_in/library/mod.rs',
    'ui/widgets/mod.rs',
    'ui/widgets/progress_icon.rs',
    'ui/widgets/sid_login/mod.rs',
    'ui/widgets/download_manager/mod.rs',
    'ui/widgets/download_manager/download_item.rs',
    'ui/mod.rs',
    'ui/messages.rs',
    'config.rs',
)

sources = [cargo_sources, rust_sources]

custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: bindir,
  depends: resources,
  command: [
    cargo_script,
    meson.build_root(),
    meson.source_root(),
    '@OUTPUT@',
    profile,
    meson.project_name(),
  ]
)
