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

sources = files(
    'window.rs',
    'tools/asset_info.rs',
    'tools/cache.rs',
    'tools/or.rs',
    'tools/mod.rs',
    'application.rs',
    'main.rs',
    'models/asset_model.rs',
    'models/row_data.rs',
    'models/mod.rs',
    'api_data.rs',
    'ui/assets.rs',
    'ui/epic_assets.rs',
    'ui/update.rs',
    'ui/configuration.rs',
    'ui/authentication.rs',
    'ui/widgets/preferences/window.rs',
    'ui/widgets/preferences/dir_row.rs',
    'ui/widgets/preferences/mod.rs',
    'ui/widgets/logged_in/mod.rs',
    'ui/widgets/logged_in/category.rs',
    'ui/widgets/logged_in/asset.rs',
    'ui/widgets/mod.rs',
    'ui/widgets/sid_login/mod.rs',
    'ui/images.rs',
    'ui/mod.rs',
    'ui/messages.rs',
    'ui/download_manifest.rs',
    'application/imp.rs',
    'window/imp.rs',
    'config.rs',
    'download/images.rs',
    'download/chunks.rs',
    'download/mod.rs'
)

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
