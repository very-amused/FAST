project('FAST', 'rust',  'c',
	version : '0.0.1', meson_version : '>=1.9.0',
	license : 'GPL-3.0-only',
	license_files : ['LICENSE'])

py = find_program('python3')

env = environment()
env.set('MESON_PROJECT_SOURCE_ROOT', meson.project_source_root())
env.set('MESON_PROJECT_BUILD_ROOT', meson.project_build_root())

if build_machine.system() == 'windows'
	outfile = 'libfast.lib'
else
	outfile = 'libfast.a'
endif

custom_target('fast',
	input : [],
	output : outfile,
	build_by_default : true,
	build_always_stale : true,
	command : [py, meson.project_source_root() / 'build_release.py', '@OUTPUT@'],
	env : env
	)
