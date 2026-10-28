/* OpenFrame design reference — screen-only preview helpers.
   Builds a toolbar (never printed) with one checkbox per optional section
   (elements carrying data-toggle="Label") and a Print / Save as PDF button.
   Nothing here affects the printed document. */
(function () {
  var bar = document.createElement("div");
  bar.className = "preview-bar no-print";
  var name = (document.querySelector('meta[name="of-doc"]') || {}).content || document.title;
  var strong = document.createElement("strong");
  strong.textContent = "OpenFrame design reference · " + name;
  bar.appendChild(strong);
  var groups = {};
  Array.prototype.forEach.call(document.querySelectorAll("[data-toggle]"), function (el) {
    var k = el.getAttribute("data-toggle");
    (groups[k] = groups[k] || []).push(el);
  });
  Object.keys(groups).forEach(function (label) {
    var l = document.createElement("label");
    var cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = !groups[label][0].hasAttribute("hidden");
    cb.addEventListener("change", function () {
      groups[label].forEach(function (el) { el.hidden = !cb.checked; });
    });
    l.appendChild(cb);
    l.appendChild(document.createTextNode(label));
    bar.appendChild(l);
  });
  var b = document.createElement("button");
  b.type = "button";
  b.textContent = "Print / Save as PDF";
  b.addEventListener("click", function () { window.print(); });
  bar.appendChild(b);
  document.body.insertBefore(bar, document.body.firstChild);

  /* on-screen sample of the running footer (the printed footer is drawn in the page margin) */
  var doc = document.querySelector(".doc");
  if (doc) {
    var logo = (document.querySelector('meta[name="of-logo"]') || {}).content || "../../00_SYSTEM/assets/openframe-logo.png";
    var f = document.createElement("div");
    f.className = "screen-footer-sample";
    f.innerHTML = '<div><img src="' + logo + '" alt="OpenFrame Studio"><div>Created using OpenFrame</div></div>' +
      '<div>Page X of Y — added on every printed page</div>';
    doc.appendChild(f);
  }
})();
