/* Home hero mosaic, a film of 6.5 seconds played once on the first visit of a session and on a reload: featured photographs take
   turns in the centre, their neighbours appear around them, the view pulls back to a dense wall, then every tile leaves the
   screen and the headline stands alone. Every tile's place is worked out from the time alone, so skipping is only a change
   of time. People who ask for less motion, save-data, a slow connection and repeat visits get the headline with no photographs,
   and those never download. */
var root = document.querySelector('[data-mosaic]');
if (!root) return;
var doc = document.documentElement;
var mos = root.querySelector('.mos'), copy = root.querySelector('.hero-copy'), btn = root.querySelector('.mos-skip');
var FEATURED = parseInt(mos.getAttribute('data-featured'), 10), DOUBLES = parseInt(mos.getAttribute('data-double'), 10);
/* the timeline, in seconds. The page splash takes about 1.2 seconds first, so a visitor sees the words alone before 8 seconds. */
var SWAP = 0.4, POP = FEATURED * SWAP, POP_SPREAD = 1.7, POP_LEN = 0.45, ZOOM = POP + 0.3, ZOOM_END = 4.2;
var BREATHE = 4.3, BREATHE_END = 4.55, EXPAND = 4.55, COPY_AT = 5.8, END = 6.5;

var clamp = function (v) { return Math.min(1, Math.max(0, v)); };
var lerp = function (a, b, k) { return a + (b - a) * k; };
var inOut = function (k) { return k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2; };
var out3 = function (k) { return 1 - Math.pow(1 - k, 3); };
var out4 = function (k) { return 1 - Math.pow(1 - k, 4); };
/* a seeded generator: the same wall on every load */
function seeded(seed) {
  return function () {
    seed = (seed + 0x6d2b79f5) | 0;
    var v = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    v = (v + Math.imul(v ^ (v >>> 7), 61 | v)) ^ v;
    return ((v ^ (v >>> 14)) >>> 0) / 4294967296;
  };
}

var W = 0, H = 0, zoomFrom = 1, feature = 0, tiles = [];

/* the dense wall: a grid with an even number of columns and rows so the first photograph sits exactly in the middle */
function layout() {
  var box = root.getBoundingClientRect(), rand = seeded(7), n = tiles.length;
  W = box.width; H = box.height;
  var budget = 4 + DOUBLES * 4 + (n - 1 - DOUBLES);
  var cell = Math.sqrt(W * H / budget), cols, rows;
  for (;;) {
    cols = Math.ceil(W / cell); rows = Math.ceil(H / cell);
    cols += cols % 2; rows += rows % 2;
    if (cols * rows <= budget) break;
    cell *= 1.03;
  }
  var gap = Math.max(3, cell * 0.04), used = {};
  var key = function (c, r) { return c + ',' + r; };
  var free = function (c, r) { return c >= 0 && r >= 0 && c < cols && r < rows && !used[key(c, r)]; };
  var take = function (t, c, r, span) {
    for (var a = 0; a < span; a++) for (var b = 0; b < span; b++) used[key(c + a, r + b)] = 1;
    t.wx = (c + span / 2 - cols / 2) * cell; t.wy = (r + span / 2 - rows / 2) * cell; t.dense = span * cell - gap;
    t.placed = true;
  };
  tiles.forEach(function (t) { t.placed = false; });
  take(tiles[0], cols / 2 - 1, rows / 2 - 1, 2);
  /* double tiles use up the cells single tiles cannot fill */
  var want = Math.max(0, Math.min(DOUBLES, Math.ceil((cols * rows - 4 - (n - 1)) / 3)));
  for (var d = 0, tries = 0; d < want && tries < 400; tries++) {
    var c = Math.floor(rand() * (cols - 1)), r = Math.floor(rand() * (rows - 1));
    if (free(c, r) && free(c + 1, r) && free(c, r + 1) && free(c + 1, r + 1)) take(tiles[FEATURED + d++], c, r, 2);
  }
  var cells = [];
  for (var y = 0; y < rows; y++) for (var x = 0; x < cols; x++) {
    if (free(x, y)) cells.push({ x: x, y: y, o: Math.hypot(x + 0.5 - cols / 2, y + 0.5 - rows / 2) + rand() * 0.8 });
  }
  cells.sort(function (a, b) { return a.o - b.o; });
  var i = 0;
  tiles.forEach(function (t) { if (!t.placed && i < cells.length) { take(t, cells[i].x, cells[i].y, 1); i++; } });
  var far = 1;
  tiles.forEach(function (t) {
    if (!t.placed) return;
    t.dist = Math.hypot(t.wx, t.wy);
    far = Math.max(far, t.dist);
  });
  feature = Math.min(H * 0.62, W * 0.8);
  zoomFrom = Math.max(1, feature / tiles[0].dense);
  tiles.forEach(function (t, n2) {
    if (!t.placed) return;
    t.popAt = n2 === 0 ? 0 : POP + POP_SPREAD * (t.dist / far);
    /* each tile leaves straight outward, far enough to clear the edge of the screen entirely */
    var a = t.dist > 1 ? Math.atan2(t.wy, t.wx) : rand() * 6.283, clear = t.dense * zoomFrom;
    var dx = Math.abs(Math.cos(a)), dy = Math.abs(Math.sin(a));
    var reach = Math.min(dx > 0.01 ? (W / 2 + clear) / dx : Infinity, dy > 0.01 ? (H / 2 + clear) / dy : Infinity);
    t.ex = Math.cos(a) * reach; t.ey = Math.sin(a) * reach;
    /* drawn at its largest size and scaled down, so a tile stays sharp at every step */
    t.base = Math.round(Math.max(t.dense * zoomFrom, n2 < FEATURED ? feature : 0));
    t.el.style.width = t.el.style.height = t.base + 'px';
  });
}

function camera(t) {
  var z = Math.exp(lerp(Math.log(zoomFrom), 0, inOut(clamp((t - ZOOM) / (ZOOM_END - ZOOM)))));
  return z * (1 - 0.04 * out4(clamp((t - BREATHE) / (BREATHE_END - BREATHE))));
}

function draw(t) {
  var cam = camera(Math.min(t, EXPAND)), span = END - EXPAND;
  var slot = Math.min(FEATURED - 1, Math.floor(t / SWAP)), shown = FEATURED - 1 - slot;
  tiles.forEach(function (tile) {
    var visible = tile.placed, size = 0, x = 0, y = 0;
    if (visible && t < POP) {
      /* the featured photographs take turns, each settling very slightly as it shows */
      visible = tile.i === shown;
      size = feature * (0.97 + 0.03 * out3(clamp(t / SWAP - slot)));
    } else if (visible) {
      size = tile.dense * cam * out3(clamp((t - tile.popAt) / POP_LEN));
      x = tile.wx * cam; y = tile.wy * cam;
      if (t > EXPAND) {
        /* the middle opens first, then the wall follows it off the screen */
        var wait = 0.3 * span * Math.min(1, tile.dist * cam / (W * 0.6));
        var k = inOut(Math.pow(clamp((t - EXPAND - wait) / (0.7 * span)), 1.8));
        x = lerp(x, tile.ex, k); y = lerp(y, tile.ey, k);
      }
    }
    visible = visible && size > 0.5;
    tile.el.classList.toggle('show', visible);
    tile.el.classList.toggle('front', tile.i === 0 && t < EXPAND);
    if (visible) {
      tile.el.style.transform = 'translate3d(' + (W / 2 + x - tile.base / 2).toFixed(2) + 'px,' + (H / 2 + y - tile.base / 2).toFixed(2) + 'px,0) scale(' + (size / tile.base).toFixed(4) + ')';
    }
  });
}

var time = 0, last = 0, raf = 0, ended = false, rate = 1;
var SKIP_ON = ['pointerdown', 'keydown', 'wheel', 'touchstart'];
function showCopy() { root.classList.remove('mos-intro'); }
/* the tiles clear three times as fast and the words fade in just behind them, so the headline never has to be read over photographs */
function skip() { if (time < EXPAND) time = EXPAND; rate = 3; root.classList.add('mos-skipped'); showCopy(); }
/* the film is over: nothing of it is left on the page */
function end() {
  if (ended) return;
  ended = true;
  if (raf) cancelAnimationFrame(raf);
  SKIP_ON.forEach(function (n) { removeEventListener(n, skip); });
  copy.removeEventListener('focusin', skip);
  showCopy();
  mos.remove();
  if (btn) btn.remove();
}
function tick(now) {
  /* a hidden tab stops the frames; the step cap keeps the film from jumping when it comes back, and is wide enough that a
     slow machine (down to 4 frames a second) does not stretch the film */
  time += rate * Math.max(0, Math.min(0.25, (now - last) / 1000));
  last = now;
  if (time >= COPY_AT) showCopy();
  if (time >= END) { end(); return; }
  draw(time);
  raf = requestAnimationFrame(tick);
}

var nav = performance.getEntriesByType && performance.getEntriesByType('navigation')[0];
var repeat = doc.classList.contains('ready') && !(nav && nav.type === 'reload');
var saveData = navigator.connection && navigator.connection.saveData;

if (matchMedia('(prefers-reduced-motion: reduce)').matches || repeat || saveData) {
  end();
} else {
  var imgs = [].slice.call(mos.querySelectorAll('img'));
  tiles = imgs.map(function (el, i) { return { el: el, i: i }; });
  root.classList.add('mos-intro');
  mos.classList.add('on');
  imgs.forEach(function (el) { el.loading = 'eager'; });
  /* the photograph on screen first is the page's largest paint, so it jumps the queue ahead of the other photographs and the app frames */
  imgs[FEATURED - 1].fetchPriority = 'high';
  layout();
  draw(0);
  var decoded = imgs.map(function (el) { return el.decode ? el.decode().catch(function () {}) : Promise.resolve(); });
  var splashGone = new Promise(function (ok) {
    (function poll() { if (doc.classList.contains('ready')) ok(); else setTimeout(poll, 40); })();
  });
  /* a slow connection skips the film rather than show empty tiles */
  Promise.race([
    Promise.all(decoded.concat([splashGone])).then(function () { return true; }),
    new Promise(function (ok) { setTimeout(function () { ok(false); }, 5000); }),
  ]).then(function (ready) {
    if (!ready) { end(); return; }
    SKIP_ON.forEach(function (n) { addEventListener(n, skip, { passive: true }); });
    copy.addEventListener('focusin', skip);
    if (btn) { btn.hidden = false; btn.addEventListener('click', skip); }
    var startWidth = innerWidth;
    addEventListener('resize', function () {
      /* a phone's address bar changes the height only; a real resize ends the film */
      if (innerWidth !== startWidth) end();
    });
    last = performance.now();
    raf = requestAnimationFrame(tick);
  });
}
