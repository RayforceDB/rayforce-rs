---
hide:
  - navigation
  - toc
---

<div class="hero-section" markdown>
<div class="hero-content" markdown>

<div class="hero-text scroll-fade-in" markdown>
# Experience Database<br>**Like Never Before** {.scroll-fade-in}

<div class="hero-description scroll-fade-in">
  <span class="hero-description-faded">Get sub-millisecond <a href="https://rayforcedb.com" class="hero-subtitle-logo-link"><img src="assets/logo_light_full.svg" alt="RayforceDB" class="hero-subtitle-logo hero-subtitle-logo--dark"><img src="assets/logo_dark_full.svg" alt="RayforceDB" class="hero-subtitle-logo hero-subtitle-logo--light"></a> performance on analytical workloads through columnar storage and SIMD vectorization — all from safe, zero-copy <strong>Rust</strong>.</span>
  <br>

</div>

<div class="hero-buttons scroll-fade-in">
<a href="content/get-started/overview.html" class="md-button md-button--primary hero-cta-button">Get Started
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
<line x1="5" y1="12" x2="19" y2="12"></line>
<polyline points="12 5 19 12 12 19"></polyline>
</svg>
</a>
<a href="https://github.com/RayforceDB/rayforce-rs" class="md-button hero-demo-button">
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
<path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"></path>
</svg>
View on GitHub
</a>
</div>

<div class="hero-socials scroll-fade-in">
<a href="https://github.com/RayforceDB/rayforce-rs" title="GitHub" class="hero-social-link">
  <img src="assets/github-logo.svg" alt="GitHub" class="hero-social-icon">
</a>
<a href="https://crates.io/crates/rayforce" title="crates.io" class="hero-social-link">
  <img src="assets/rust_logo.svg" alt="crates.io" class="hero-social-icon">
</a>
<a href="https://rayforcedb.zulipchat.com/#narrow/channel/549008-Discuss" title="Zulip Chat" class="hero-social-link">
  <img src="assets/zulip-logo.svg" alt="Zulip Chat" class="hero-social-icon">
</a>
<a href="content/license.html" title="MIT License" class="hero-social-link">
  <img src="assets/mit-logo.svg" alt="MIT License" class="hero-social-icon">
</a>
</div>
</div>

<div class="hero-logo-wrapper scroll-fade-in">
<img src="assets/logo_dark_full.svg" alt="Rayforce-RS" class="hero-logo light-only">
<img src="assets/logo_light_full.svg" alt="Rayforce-RS" class="hero-logo dark-only">
</div>

</div>
</div>

<div class="bento-section scroll-fade-in">
<div class="bento-grid-static">

<a href="content/get-started/technical-details.html" class="bento-card-link">
<div class="bento-card bento-card-brand-gold">
<div class="bento-card-content">
<h3><strong>Zero Overhead</strong></h3>
<p>Calls the RayforceDB core's C API directly — no marshalling shim. Reads are zero-copy: a numeric column is a <code>&amp;[T]</code> slice, not a per-element copy.</p>
<div class="hero-socials">
<a href="content/get-started/technical-details.html" title="Technical Details" class="hero-social-link">
  <img src="assets/wrench-icon.svg" alt="Technical Details" class="hero-social-icon">
</a>
</div>
</div>
</div>
</a>

<a href="content/get-started/technical-details.html" class="bento-card-link">
<div class="bento-card bento-card-wide bento-card-brand-navy-light">
<div class="bento-card-content">
<h3><strong>Performance</strong></h3>
<p>Columnar storage and SIMD vectorization deliver consistent sub-millisecond latency on analytical workloads. Build a column in a single <code>memcpy</code>; read it back as a borrow.</p>
</div>
</div>
</a>

<a href="https://rayforcedb.com" class="bento-card-link">
<div class="bento-card bento-card-tall bento-card-brand-navy">
<div class="bento-card-content">
<h3><strong>Ecosystem</strong></h3>
<p>RayforceDB spans <strong>multiple languages and in-house tools</strong>, from the C core to editor tooling — enabling high-performance, distributed solutions.</p>
<div class="hero-socials">
<a href="https://core.rayforcedb.com" title="RayforceDB" class="hero-social-link-light">
  <img src="assets/logo_gray.svg" alt="RayforceDB" class="hero-social-icon-light">
</a>
<a href="https://vscode.rayforcedb.com" title="Rayforce-VSCode" class="hero-social-link-light">
  <img src="assets/vscode.svg" alt="Rayforce-VSCode" class="hero-social-icon-light">
</a>
<a href="https://github.com/RayforceDB/rayforce-rs" title="Rayforce-RS" class="hero-social-link-light">
  <img src="assets/rust_logo.svg" alt="Rayforce-RS" class="hero-social-icon-light">
</a>
</div>
</div>
</div>
</a>

<a href="content/documentation/query-guide/overview.html" class="bento-card-link">
<div class="bento-card bento-card-brand-gold-light">
<div class="bento-card-content">
<h3><strong>Fluent Queries</strong></h3>
<p>Chainable query builders that <strong>read like the operation</strong>. Operator overloads for arithmetic, methods for comparisons, aggregations either way.</p>
</div>
</div>
</a>

<a href="content/documentation/data-types/overview.html" class="bento-card-link">
<div class="bento-card bento-card-brand-gold">
<div class="bento-card-content">
<h3><strong>Type-Safe &amp; Zero-Copy</strong></h3>
<p>A full value model — atoms, vectors, lists, dicts, tables — with <code>ToValue</code>/<code>FromValue</code> conversions and optional <code>chrono</code> temporals.</p>
</div>
</div>
</a>

<a href="content/documentation/ipc.html" class="bento-card-link">
<div class="bento-card bento-card-wide bento-card-brand-navy">
<div class="bento-card-content">
<h3><strong>TCP Servers</strong></h3>
<p>Execute queries over TCP with low latency. <strong>Connect to servers</strong>, organize data in a <strong>distributed manner</strong>, and build distributed processing applications.</p>
</div>
</div>
</a>

</div>
</div>

<div class="pylab-section scroll-fade-in" markdown>
<div class="pylab-content" markdown>
<h2><strong>Rayforce-RS</strong> in a few lines</h2>
<p>Build a table, run a grouped aggregation, and print it — all from safe Rust.</p>

```rust
use rayforce::{col, sum, Runtime, Table, Value};

let _rt = Runtime::new()?;

let t = Table::new(
    &["sym", "price", "size"],
    &[
        Value::sym_vec(&["AAPL", "MSFT", "AAPL", "GOOG"]),
        Value::vec(&[100.0f64, 200.0, 110.0, 300.0]),
        Value::vec(&[10i64, 20, 30, 40]),
    ],
)?;

let totals = t
    .select()
    .agg("total", sum(col("size")))
    .filter(col("price").gt(150.0))
    .by("sym")
    .execute()?;

println!("{totals}");
```

</div>
</div>

<style>
.scroll-fade-in {
  opacity: 0;
  transform: translateY(20px);
  transition: opacity 0.6s ease-out, transform 0.6s ease-out;
}
.scroll-fade-in.visible {
  opacity: 1;
  transform: translateY(0);
}
.hero-content > .scroll-fade-in:first-child {
  opacity: 1 !important;
  transform: translateY(0) !important;
  transition-delay: 0s !important;
}
.hero-content > h1.scroll-fade-in,
.hero-content > .hero-description.scroll-fade-in { transition-delay: 0.2s; }
.hero-content > .hero-buttons.scroll-fade-in { transition-delay: 0.3s; }
.hero-content > .hero-socials.scroll-fade-in { transition-delay: 0.4s; }
</style>

<script>
document.addEventListener('DOMContentLoaded', function() {
  const observer = new IntersectionObserver(function(entries) {
    entries.forEach(entry => {
      if (entry.isIntersecting) entry.target.classList.add('visible');
    });
  }, { threshold: 0.1, rootMargin: '0px 0px -50px 0px' });

  document.querySelectorAll('.scroll-fade-in').forEach(el => {
    if (!el.matches('.hero-content > .scroll-fade-in:first-child')) {
      observer.observe(el);
    } else {
      el.classList.add('visible');
    }
  });
});
</script>
