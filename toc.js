// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="components/_overview.html"><strong aria-hidden="true">2.</strong> High-level overview</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="components/domains.html"><strong aria-hidden="true">2.1.</strong> Domains</a></li><li class="chapter-item expanded "><a href="components/accounts.html"><strong aria-hidden="true">2.2.</strong> Accounts</a></li><li class="chapter-item expanded "><a href="components/libraries_and_functions.html"><strong aria-hidden="true">2.3.</strong> Libraries and Functions</a></li><li class="chapter-item expanded "><a href="components/programs_and_authorizations.html"><strong aria-hidden="true">2.4.</strong> Programs and Authorizations</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/_overview.html"><strong aria-hidden="true">3.</strong> Authorizations &amp; Processors</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/assumptions.html"><strong aria-hidden="true">3.1.</strong> Assumptions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/processor.html"><strong aria-hidden="true">3.2.</strong> Processor Contract</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization.html"><strong aria-hidden="true">3.3.</strong> Authorization Contract</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/authorization_instantiation.html"><strong aria-hidden="true">3.3.1.</strong> Instantiation</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_owner_actions.html"><strong aria-hidden="true">3.3.2.</strong> Owner Actions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_user_actions.html"><strong aria-hidden="true">3.3.3.</strong> User Actions</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/callbacks.html"><strong aria-hidden="true">3.4.</strong> Callbacks</a></li></ol></li><li class="chapter-item expanded "><a href="libraries/_overview.html"><strong aria-hidden="true">4.</strong> Libraries</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="libraries/astroport-lper.html"><strong aria-hidden="true">4.1.</strong> Astroport LPer</a></li><li class="chapter-item expanded "><a href="libraries/astroport-withdrawer.html"><strong aria-hidden="true">4.2.</strong> Astroport Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/forwarder.html"><strong aria-hidden="true">4.3.</strong> Forwarder</a></li><li class="chapter-item expanded "><a href="libraries/generic-ibc-transfer.html"><strong aria-hidden="true">4.4.</strong> Generic IBC Transfer</a></li><li class="chapter-item expanded "><a href="libraries/neutron-ibc-transfer.html"><strong aria-hidden="true">4.5.</strong> Neutron IBC Transfer</a></li><li class="chapter-item expanded "><a href="libraries/osmosis-cl-lper.html"><strong aria-hidden="true">4.6.</strong> Osmosis CL LPer</a></li><li class="chapter-item expanded "><a href="libraries/osmosis-cl-withdrawer.html"><strong aria-hidden="true">4.7.</strong> Osmosis CL Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/osmosis-gamm-lper.html"><strong aria-hidden="true">4.8.</strong> Osmosis GAMM LPer</a></li><li class="chapter-item expanded "><a href="libraries/osmosis-gamm-withdrawer.html"><strong aria-hidden="true">4.9.</strong> Osmosis GAMM Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/reverse-splitter.html"><strong aria-hidden="true">4.10.</strong> Reverse Splitter</a></li><li class="chapter-item expanded "><a href="libraries/splitter.html"><strong aria-hidden="true">4.11.</strong> Splitter</a></li></ol></li><li class="chapter-item expanded "><a href="examples/_overview.html"><strong aria-hidden="true">5.</strong> Examples</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="examples/token_swap.html"><strong aria-hidden="true">5.1.</strong> Token Swap</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
