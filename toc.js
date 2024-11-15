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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="background.html"><strong aria-hidden="true">1.</strong> Background</a></li><li class="chapter-item expanded "><a href="components/overview.html"><strong aria-hidden="true">2.</strong> Component Overview</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="components/accounts_and_libraries.html"><strong aria-hidden="true">2.1.</strong> Accounts and Libraries</a></li><li class="chapter-item expanded "><a href="components/authorizations_and_execution.html"><strong aria-hidden="true">2.2.</strong> Authorizations and Execution</a></li><li class="chapter-item expanded "><a href="components/programs.html"><strong aria-hidden="true">2.3.</strong> Programs</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/overview.html"><strong aria-hidden="true">3.</strong> Authorizations &amp; Processors</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/assumptions.html"><strong aria-hidden="true">3.1.</strong> Assumptions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/processor.html"><strong aria-hidden="true">3.2.</strong> Processor Contract</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization.html"><strong aria-hidden="true">3.3.</strong> Authorization Contract</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/authorization_instantiation.html"><strong aria-hidden="true">3.3.1.</strong> Instantiation</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_owner_actions.html"><strong aria-hidden="true">3.3.2.</strong> Owner Actions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_user_actions.html"><strong aria-hidden="true">3.3.3.</strong> User Actions</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/callbacks.html"><strong aria-hidden="true">3.4.</strong> Callbacks</a></li></ol></li><li class="chapter-item expanded "><a href="examples.html"><strong aria-hidden="true">4.</strong> Examples</a></li></ol>';
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
