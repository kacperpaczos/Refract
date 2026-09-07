#!/usr/bin/env python3

"""

https://setuptools.readthedocs.io/en/latest/setuptools.html#new-and-changed-setup-keywords
build:
python setup.py bdist

"""

from setuptools import setup, find_packages

setup(
    name='layoutswitcherlib',
    version='0.8.39',
    description='Gnome Layout Switcher',
    url='https://gitlab.manjaro.org/Chrysostomus/gnome-layout-switcher',
    download_url='https://gitlab.manjaro.org/Chrysostomus/gnome-layout-switcher',
    keywords = ["gnome", "manjaro"],
    author="Chrysostomus, papajoke, Ste74",
    python_requires='>=3.7',
    packages=['layoutswitcherlib'],
    package_dir = {'layoutswitcherlib' : 'bin/layoutswitcherlib'},
    include_package_data=True,

    data_files=[
        ('share/icons/hicolor/scalable/apps',[
            'data/gnome-layout-switcher.svg',
         ]),
        ('share/gls/pictures',[
            'data/pictures/gnomepreview.svg',
            'data/pictures/manjaropreview.svg',
            'data/pictures/tilingpreview.svg',
            'data/pictures/traditionalpreview.svg',
         ]),
        ('share/gls/schemas',[
            'data/schemas/traditional_layout',
         ]),
         ('bin/',[
            'bin/gnome-layout-switcher',
         ]),
        ('share/applications',[
            'data/desktop/gnome-layouts.desktop',
         ]),
     ]
)
