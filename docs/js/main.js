/**
 * AURA — Interactive Behavior
 * Minimal, purposeful vanilla JavaScript. No framework overhead.
 */

document.addEventListener('DOMContentLoaded', () => {
  // --- 1. Sticky Nav Behavior ---
  const nav = document.querySelector('.site-nav');
  if (nav) {
    window.addEventListener('scroll', () => {
      if (window.scrollY > 20) {
        nav.classList.add('scrolled');
      } else {
        nav.classList.remove('scrolled');
      }
    }, { passive: true });
  }

  // --- 2. Hero Video Controls ---
  const heroVideo = document.getElementById('hero-video');
  const heroTogglePlay = document.getElementById('hero-toggle-play');
  const heroToggleSound = document.getElementById('hero-toggle-sound');
  const heroPlayIcon = document.getElementById('hero-play-icon');
  const heroPlayText = document.getElementById('hero-play-text');
  const heroSoundText = document.getElementById('hero-sound-text');

  if (heroVideo) {
    // Smart view observer: pause video when scrolled completely out of view
    const videoObserver = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (!entry.isIntersecting && !heroVideo.paused) {
          heroVideo.pause();
          if (heroPlayText) heroPlayText.textContent = 'Play';
        } else if (entry.isIntersecting && heroVideo.paused && heroVideo.dataset.userPaused !== 'true') {
          heroVideo.play().catch(() => {});
          if (heroPlayText) heroPlayText.textContent = 'Pause';
        }
      });
    }, { threshold: 0.1 });

    videoObserver.observe(heroVideo);

    if (heroTogglePlay) {
      heroTogglePlay.addEventListener('click', () => {
        if (heroVideo.paused) {
          heroVideo.play();
          heroVideo.dataset.userPaused = 'false';
          if (heroPlayText) heroPlayText.textContent = 'Pause';
        } else {
          heroVideo.pause();
          heroVideo.dataset.userPaused = 'true';
          if (heroPlayText) heroPlayText.textContent = 'Play';
        }
      });
    }

    if (heroToggleSound) {
      heroToggleSound.addEventListener('click', () => {
        heroVideo.muted = !heroVideo.muted;
        if (heroSoundText) {
          heroSoundText.textContent = heroVideo.muted ? 'Sound [Muted]' : 'Sound [On]';
        }
      });
    }
  }

  // --- 3. Dynamic Auto-Theming Comparison Toggle (Section 03) ---
  const themeSwitchBtns = document.querySelectorAll('.theme-switch-btn');
  const motionImgAmber = document.getElementById('motion-img-amber');
  const motionImgViolet = document.getElementById('motion-img-violet');
  const motionThemeCaption = document.getElementById('motion-theme-caption');

  themeSwitchBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const mode = btn.getAttribute('data-theme-mode');

      themeSwitchBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');

      if (mode === 'violet') {
        if (motionImgAmber) motionImgAmber.style.display = 'none';
        if (motionImgViolet) motionImgViolet.style.display = 'block';
        if (motionThemeCaption) {
          motionThemeCaption.textContent = 'State: Dark wallpaper active → Desktop accent dynamically shifted to violet (RGB: 168, 85, 247).';
        }
      } else {
        if (motionImgAmber) motionImgAmber.style.display = 'block';
        if (motionImgViolet) motionImgViolet.style.display = 'none';
        if (motionThemeCaption) {
          motionThemeCaption.textContent = 'State: Solar field wallpaper active → Desktop accent dynamically shifted to warm gold (RGB: 245, 158, 11).';
        }
      }
    });
  });

  // --- 4. Copy-to-Clipboard Functionality ---
  const copyButtons = document.querySelectorAll('[data-copy-target]');
  copyButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const targetSelector = btn.getAttribute('data-copy-target');
      const targetElem = document.querySelector(targetSelector);
      if (targetElem) {
        const text = targetElem.innerText.trim();
        navigator.clipboard.writeText(text).then(() => {
          showToast(`COPIED: ${text}`);
        }).catch(() => {
          showToast('Failed to copy');
        });
      }
    });
  });

  // Copy CLI Reference Button
  const copyCliBtn = document.getElementById('copy-cli-all');
  if (copyCliBtn) {
    copyCliBtn.addEventListener('click', () => {
      const cliCommands = [
        'aura next',
        'aura prev',
        'aura toggle-pause',
        'aura apply ~/Pictures/Wallpapers/space.mp4',
        'aura stop',
        'aura status',
        'aura check-update',
        'aura update',
        'aura --daemon'
      ].join('\n');
      navigator.clipboard.writeText(cliCommands).then(() => {
        showToast('COPIED 9 CLI COMMANDS');
      });
    });
  }

  // --- 4b. Multi-Distro Dependency Tabs ---
  const distroTabs = document.querySelectorAll('.distro-tab-btn');
  const cmdDepsElem = document.getElementById('cmd-deps');
  const distroCommands = {
    'apt': 'sudo apt install libmpv2 ffmpeg',
    'pacman': 'sudo pacman -S mpv ffmpeg',
    'dnf': 'sudo dnf install mpv-libs ffmpeg'
  };

  distroTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const distro = tab.getAttribute('data-distro');
      distroTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      if (cmdDepsElem && distroCommands[distro]) {
        cmdDepsElem.textContent = distroCommands[distro];
      }
    });
  });

  // --- 4c. Web Installer Mode Tabs (User vs System) ---
  const installerTabs = document.querySelectorAll('.installer-tab-btn');
  const cmdInstallerElem = document.getElementById('cmd-web-installer');
  const installerCommands = {
    'user': 'curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh | bash',
    'system': 'curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh | bash -s -- --system'
  };

  installerTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const mode = tab.getAttribute('data-installer-mode');
      installerTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      if (cmdInstallerElem && installerCommands[mode]) {
        cmdInstallerElem.textContent = installerCommands[mode];
      }
    });
  });

  // --- 5. Discreet Monospace Toast ---
  let toastTimer;
  const toast = document.getElementById('discreet-toast');
  const toastText = document.getElementById('toast-text');

  function showToast(msg) {
    if (!toast || !toastText) return;
    toastText.textContent = msg;
    toast.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast.classList.remove('show');
    }, 2400);
  }

  // --- 6. Screenshot Lightbox Modal ---
  const lightbox = document.getElementById('editorial-lightbox');
  const lightboxImg = document.getElementById('lightbox-img');
  const lightboxClose = document.getElementById('lightbox-close');
  const lightboxCaption = document.getElementById('lightbox-caption');
  const inspectTargets = document.querySelectorAll('.inspectable-image');

  inspectTargets.forEach(target => {
    target.addEventListener('click', () => {
      if (lightbox && lightboxImg) {
        lightboxImg.src = target.getAttribute('src');
        lightboxImg.alt = target.getAttribute('alt') || 'Screenshot';
        if (lightboxCaption) {
          lightboxCaption.textContent = target.getAttribute('alt') || '';
        }
        lightbox.classList.add('open');
        document.body.style.overflow = 'hidden';
      }
    });
  });

  function closeLightbox() {
    if (lightbox) {
      lightbox.classList.remove('open');
      document.body.style.overflow = '';
    }
  }

  if (lightboxClose) lightboxClose.addEventListener('click', closeLightbox);
  if (lightbox) {
    lightbox.addEventListener('click', (e) => {
      if (e.target === lightbox) closeLightbox();
    });
  }
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeLightbox();
  });

  // --- 7. Mobile Navigation Toggle ---
  const mobileToggle = document.getElementById('nav-mobile-toggle');
  const mobileOverlay = document.getElementById('mobile-nav-overlay');

  if (mobileToggle && mobileOverlay) {
    mobileToggle.addEventListener('click', () => {
      const isOpen = mobileOverlay.classList.contains('open');
      if (isOpen) {
        mobileOverlay.classList.remove('open');
        mobileToggle.setAttribute('aria-expanded', 'false');
      } else {
        mobileOverlay.classList.add('open');
        mobileToggle.setAttribute('aria-expanded', 'true');
      }
    });

    mobileOverlay.querySelectorAll('a').forEach(link => {
      link.addEventListener('click', () => {
        mobileOverlay.classList.remove('open');
        mobileToggle.setAttribute('aria-expanded', 'false');
      });
    });
  }
});
