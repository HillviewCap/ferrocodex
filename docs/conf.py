# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import json
import os

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'Ferrocodex'
copyright = '2025-2026, Ferrocodex Team'
author = 'Ferrocodex Team'

# The version is read from the desktop package manifest so that the docs
# follow the application's release number without manual edits. Read the Docs
# checks out the whole repository, so the relative path resolves there too.
_package_json = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    '..', 'apps', 'desktop', 'package.json',
)
with open(_package_json, encoding='utf-8') as _f:
    release = json.load(_f)['version']
version = '.'.join(release.split('.')[:2])

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    'myst_parser',
    'sphinx_copybutton',
    'sphinx_tabs.tabs',
]

templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'sphinx_rtd_theme'
html_static_path = ['_static']
html_logo = None
html_favicon = None

# Theme options
html_theme_options = {
    'logo_only': False,
    'display_version': True,
    'prev_next_buttons_location': 'bottom',
    'style_external_links': False,
    'collapse_navigation': True,
    'sticky_navigation': True,
    'navigation_depth': 4,
    'includehidden': True,
    'titles_only': False
}

# MyST Parser configuration
myst_enable_extensions = [
    "deflist",
    "tasklist",
    "html_image",
    "colon_fence",
    "smartquotes",
    "replacements",
]

# Copy button configuration
copybutton_prompt_text = r"$ |>>> |\.\.\. "
copybutton_prompt_is_regexp = True