/**
 * AURA — Interactive Behavior
 * Minimalist, high-performance vanilla JavaScript. Inspired by Noctis.
 */

document.addEventListener('DOMContentLoaded', () => {
  // --- 1. Hero Video Controls & Smart View Observer ---
  const heroVideo = document.getElementById('hero-video');
  const heroTogglePlay = document.getElementById('hero-toggle-play');
  const heroToggleSound = document.getElementById('hero-toggle-sound');
  const heroPlayText = document.getElementById('hero-play-text');
  const heroSoundText = document.getElementById('hero-sound-text');

  if (heroVideo) {
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

  // --- 2. Showcase Pill Tabs Switching ---
  const showcaseTabs = document.querySelectorAll('.showcase__tab-btn');
  const showcasePanels = document.querySelectorAll('.showcase__panel');

  showcaseTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const targetId = tab.getAttribute('data-showcase-tab');

      showcaseTabs.forEach(t => {
        t.classList.remove('active');
        t.setAttribute('aria-selected', 'false');
      });
      tab.classList.add('active');
      tab.setAttribute('aria-selected', 'true');

      showcasePanels.forEach(panel => {
        panel.classList.remove('active');
      });

      const activePanel = document.getElementById(`panel-${targetId}`);
      if (activePanel) {
        activePanel.classList.add('active');
      }
    });
  });

  // --- 3. Dynamic Auto-Theming Comparison Toggle (Showcase) ---
  const themeSwitchBtns = document.querySelectorAll('.theme-pill-btn');
  const motionImgAmber = document.getElementById('motion-img-amber');
  const motionImgViolet = document.getElementById('motion-img-violet');
  const motionThemeCaption = document.getElementById('motion-theme-caption');

  themeSwitchBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const mode = btn.getAttribute('data-theme-mode');

      themeSwitchBtns.forEach(b => {
        b.classList.remove('active');
        b.setAttribute('aria-pressed', 'false');
      });
      btn.classList.add('active');
      btn.setAttribute('aria-pressed', 'true');

      if (mode === 'violet') {
        if (motionImgAmber) motionImgAmber.style.display = 'none';
        if (motionImgViolet) {
          motionImgViolet.style.display = 'block';
          motionImgViolet.style.opacity = '0';
          motionImgViolet.style.transition = 'opacity 0.25s ease';
          requestAnimationFrame(() => {
            motionImgViolet.style.opacity = '1';
          });
        }
        if (motionThemeCaption) {
          motionThemeCaption.textContent = 'State: Dark cyberpunk wallpaper active → COSMIC Desktop accent dynamically shifted to violet (RGB: 168, 85, 247).';
        }
      } else {
        if (motionImgAmber) {
          motionImgAmber.style.display = 'block';
          motionImgAmber.style.opacity = '0';
          motionImgAmber.style.transition = 'opacity 0.25s ease';
          requestAnimationFrame(() => {
            motionImgAmber.style.opacity = '1';
          });
        }
        if (motionImgViolet) motionImgViolet.style.display = 'none';
        if (motionThemeCaption) {
          motionThemeCaption.textContent = 'State: Solar field wallpaper active → COSMIC Desktop accent dynamically shifted to warm gold (RGB: 245, 158, 11).';
        }
      }
    });
  });

  // --- 4. Distro Dependency Tabs (Download Card) ---
  const distroTabs = document.querySelectorAll('.distro-tab');
  const cmdDepsElem = document.getElementById('cmd-deps-main');
  const distroCommands = {
    'pacman': 'sudo pacman -S --needed mpv ffmpeg',
    'apt': 'sudo apt install -y libmpv2 ffmpeg',
    'dnf': 'sudo dnf install -y mpv-libs ffmpeg-free',
    'zypper': 'sudo zypper install -y mpv ffmpeg'
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

  // --- 5. Copy-to-Clipboard Functionality ---
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

  // --- 5.1 CLI Console Interactive Features ---
  const cliFilterBtns = document.querySelectorAll('.cli-filter-btn');
  const cliRows = document.querySelectorAll('.cli-row');
  const copyCliAllBtn = document.getElementById('copy-cli-all');
  const rowCopyBtns = document.querySelectorAll('[data-copy-cmd]');

  if (cliFilterBtns.length > 0 && cliRows.length > 0) {
    cliFilterBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const filter = btn.getAttribute('data-filter');

        cliFilterBtns.forEach(b => {
          b.classList.remove('active');
          b.setAttribute('aria-selected', 'false');
        });
        btn.classList.add('active');
        btn.setAttribute('aria-selected', 'true');

        cliRows.forEach(row => {
          const category = row.getAttribute('data-category');
          if (filter === 'all' || category === filter) {
            row.classList.remove('hidden');
          } else {
            row.classList.add('hidden');
          }
        });
      });
    });
  }

  if (copyCliAllBtn) {
    copyCliAllBtn.addEventListener('click', () => {
      const cliCommands = [
        'aura switcher',
        'aura next',
        'aura prev',
        'aura toggle-pause',
        'aura',
        'aura apply ~/Pictures/Wallpapers/space.mp4',
        'aura stop',
        'aura status',
        'aura check-update',
        'aura update',
        'aura --daemon',
        'aura --version',
        'aura help'
      ].join('\n');

      navigator.clipboard.writeText(cliCommands).then(() => {
        showToast('COPIED 13 CLI COMMANDS');
      }).catch(() => {
        showToast('Failed to copy commands');
      });
    });
  }

  if (rowCopyBtns.length > 0) {
    rowCopyBtns.forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const cmd = btn.getAttribute('data-copy-cmd');
        if (cmd) {
          navigator.clipboard.writeText(cmd).then(() => {
            showToast(`COPIED: ${cmd}`);
          }).catch(() => {
            showToast('Failed to copy');
          });
        }
      });
    });
  }

  // --- 6. Discreet Monospace Toast ---
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

  // --- 7. Screenshot Lightbox Modal ---
  const lightbox = document.getElementById('editorial-lightbox');
  const lightboxImg = document.getElementById('lightbox-img');
  const lightboxClose = document.getElementById('lightbox-close');
  const inspectTargets = document.querySelectorAll('.inspectable-image');

  inspectTargets.forEach(target => {
    target.addEventListener('click', () => {
      if (lightbox && lightboxImg) {
        lightboxImg.src = target.getAttribute('src');
        lightboxImg.alt = target.getAttribute('alt') || 'Screenshot';
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

  // --- 8. Mobile Navigation Toggle ---
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

  // --- 9. Scroll-Triggered Reveal Animations ---
  const revealElements = document.querySelectorAll('.reveal-on-scroll');
  if (revealElements.length > 0) {
    const revealObserver = new IntersectionObserver((entries, observer) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-visible');
          observer.unobserve(entry.target);
        }
      });
    }, {
      rootMargin: '0px 0px -40px 0px',
      threshold: 0.08
    });

    revealElements.forEach(el => revealObserver.observe(el));
  }
});
