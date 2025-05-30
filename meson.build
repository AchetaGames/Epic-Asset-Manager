project('epic_asset_manager',
        'rust',
        version: '3.8.6',
        license: 'MIT',
  meson_version: '>= 0.59')

i18n = import('i18n')
gnome = import('gnome')

base_id = 'io.github.achetagames.epic_asset_manager'

dependency('glib-2.0', version: '>= 2.76')
dependency('gio-2.0', version: '>= 2.76')
dependency('gtk4', version: '>= 4.0')
dependency(
  'libadwaita-1', version: '>= 1.5',
  fallback: ['libadwaita', 'libadwaita_dep'],
  default_options: ['tests=false', 'examples=false', 'vapi=false']
)


glib_compile_resources = find_program('glib-compile-resources', required: true)
glib_compile_schemas = find_program('glib-compile-schemas', required: true)
desktop_file_validate = find_program('desktop-file-validate', required: false)
appstream_util = find_program('appstream-util', required: false)
cargo = find_program('cargo', required: true)

    cargo_script = find_program('build-aux/cargo.sh')
    cargo_sources = files(
      'Cargo.toml',
    )

version = meson.project_version()

if get_option('windows')
    prefix = ''
    bindir = ''
    localedir = 'locale'
    datadir = 'data'
    pkgdatadir = datadir
    iconsdir = datadir / 'icons'
    podir = meson.project_source_root() / 'po'
else
    prefix = get_option('prefix')
    bindir = prefix / get_option('bindir')
    localedir = prefix / get_option('localedir')

    datadir = prefix / get_option('datadir')
    pkgdatadir = datadir / meson.project_name()
    iconsdir = datadir / 'icons'
    podir = meson.project_source_root() / 'po'
endif
gettext_package = meson.project_name()

if get_option('profile') == 'development'
  profile = 'Devel'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD', check: true).stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format(vcs_tag)
  endif
  application_id = '@0@.@1@'.format(base_id, profile)
else
  profile = ''
  version_suffix = ''
  application_id = '@0@'.format(base_id)
endif

meson.add_dist_script(
  'build-aux/dist-vendor.sh',
  meson.project_build_root() / 'meson-dist' / meson.project_name() + '-' + version,
  meson.project_source_root()
)

if get_option('profile') == 'development'
  # Setup pre-commit hook for ensuring coding style is always consistent
  message('Setting up git pre-commit hook..')
  run_command('cp', '-f', 'hooks/pre-commit.hook', '.git/hooks/pre-commit', check: true)
endif

subdir('data')
subdir('po')
subdir('src')

gnome.post_install(
  gtk_update_icon_cache: true,
  glib_compile_schemas: true,
  update_desktop_database: true,
)
