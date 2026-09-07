project('gnome-shell-extension-snapd-prompting',
  version : '2',
  meson_version : '>= 0.60',
  license: 'GPL-2.0-or-later',
)

gnome = import ('gnome')
i18n = import('i18n')

if get_option('local_install').auto()
    local_install = false
    if get_option('prefix') == '/usr/local'
        local_install = run_command(find_program('id'), '-u').stdout().strip() != '0'
    endif
else
    local_install = get_option('local_install').enabled()
endif

sh = find_program('sh')
uuid = 'snapd-prompting@canonical.com'

if local_install
    prefix = run_command(sh, '-c', 'echo $HOME').stdout().strip() / '.local'
else
    prefix = get_option('prefix')
endif

extension_dir = prefix / 'share' / 'gnome-shell' / 'extensions'

if local_install
    locale_dir = extension_dir / 'locale'
else
    locale_dir = prefix / get_option('localedir')
endif

install_subdir(uuid,
    install_dir: extension_dir,
    exclude_files: [
        '.eslintrc.yml',
        'eslint.config.mjs',
    ],
    exclude_directories: [
        'lint',
    ],
)

summary({
    'Local installation': local_install,
    'Extension UUID': uuid,
  },
  section: 'Configuration')

summary({
  'Extension directory': extension_dir,
  },
  section: 'Directories')
