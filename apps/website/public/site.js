// Progressive enhancement for the download section.
// Without JavaScript the page links to the release fixed in the HTML.
// With it, the page points at the latest GitHub release and puts the
// visitor's own platform first.
(function () {
  'use strict';

  var REPO = 'HillviewCap/ferrocodex';
  var RELEASES_API = 'https://api.github.com/repos/' + REPO + '/releases/latest';

  // Asset filename patterns per download link, keyed by data-asset.
  var PATTERNS = {
    'windows-msi': /_x64_en-US\.msi$/,
    'windows-exe': /_x64-setup\.exe$/,
    'macos-arm': /_aarch64\.dmg$/,
    'macos-intel': /_x64\.dmg$/,
    'linux-appimage': /_amd64\.AppImage$/,
    'linux-deb': /_amd64\.deb$/,
    'linux-rpm': /\.x86_64\.rpm$/
  };

  var PLATFORM_LABEL = {
    windows: 'Download for Windows',
    macos: 'Download for macOS',
    linux: 'Download for Linux'
  };

  function detectPlatform() {
    var ua = navigator.userAgent || '';
    var data = navigator.userAgentData;
    var p = (data && data.platform) || navigator.platform || '';
    if (/Windows/i.test(p) || /Windows/i.test(ua)) return 'windows';
    if (/Mac/i.test(p) || /Macintosh/i.test(ua)) return 'macos';
    if (/Linux/i.test(p) || /Linux|X11/i.test(ua)) return 'linux';
    return null;
  }

  function applyPlatform(platform) {
    if (!platform) return;
    var hero = document.querySelector('[data-hero-download]');
    if (hero) {
      hero.textContent = PLATFORM_LABEL[platform];
      hero.setAttribute('href', '#download-' + platform);
    }
    var rows = document.querySelectorAll('[data-platform]');
    for (var i = 0; i < rows.length; i++) {
      rows[i].classList.toggle('is-current', rows[i].getAttribute('data-platform') === platform);
    }
    var current = document.querySelector('[data-platform="' + platform + '"]');
    if (current && current.parentNode) {
      current.parentNode.insertBefore(current, current.parentNode.firstChild);
    }
  }

  function formatSize(bytes) {
    if (bytes >= 1024 * 1024) return Math.round(bytes / (1024 * 1024)) + ' MB';
    return Math.round(bytes / 1024) + ' KB';
  }

  function applyRelease(release) {
    if (!release || !release.assets || !release.tag_name) return;
    var version = release.tag_name.replace(/^v/, '');
    var versionNodes = document.querySelectorAll('[data-version]');
    for (var i = 0; i < versionNodes.length; i++) versionNodes[i].textContent = version;

    var links = document.querySelectorAll('[data-asset]');
    for (var j = 0; j < links.length; j++) {
      var key = links[j].getAttribute('data-asset');
      var pattern = PATTERNS[key];
      if (!pattern) continue;
      for (var k = 0; k < release.assets.length; k++) {
        var asset = release.assets[k];
        if (pattern.test(asset.name)) {
          links[j].setAttribute('href', asset.browser_download_url);
          var size = links[j].parentNode.querySelector('[data-size]');
          if (size) size.textContent = formatSize(asset.size);
          break;
        }
      }
    }
    var notes = document.querySelector('[data-release-notes]');
    if (notes && release.html_url) notes.setAttribute('href', release.html_url);
  }

  applyPlatform(detectPlatform());

  if (typeof fetch !== 'function') return;
  fetch(RELEASES_API, { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (res) { return res.ok ? res.json() : null; })
    .then(applyRelease)
    .catch(function () { /* keep the links fixed in the HTML */ });
})();
