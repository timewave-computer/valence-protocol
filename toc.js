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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="components/_overview.html"><strong aria-hidden="true">2.</strong> Valence Programs</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="components/domains.html"><strong aria-hidden="true">2.1.</strong> Domains</a></li><li class="chapter-item expanded "><a href="accounts/_overview.html"><strong aria-hidden="true">2.2.</strong> Accounts</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="accounts/base_accounts.html"><strong aria-hidden="true">2.2.1.</strong> Base Accounts</a></li><li class="chapter-item expanded "><a href="accounts/storage_accounts.html"><strong aria-hidden="true">2.2.2.</strong> Storage Accounts</a></li><li class="chapter-item expanded "><a href="accounts/interchain_accounts.html"><strong aria-hidden="true">2.2.3.</strong> Interchain Accounts</a></li></ol></li><li class="chapter-item expanded "><a href="components/libraries_and_functions.html"><strong aria-hidden="true">2.3.</strong> Libraries and Functions</a></li><li class="chapter-item expanded "><a href="components/programs_and_authorizations.html"><strong aria-hidden="true">2.4.</strong> Programs and Authorizations</a></li><li class="chapter-item expanded "><a href="components/middleware.html"><strong aria-hidden="true">2.5.</strong> Middleware</a></li></ol></li><li class="chapter-item expanded "><a href="zk-coprocessor/_overview.html"><strong aria-hidden="true">3.</strong> Valence zk-Coprocessor</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="zk-coprocessor/sparse-merkle-trees.html"><strong aria-hidden="true">3.1.</strong> Sparse Merkle Trees</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/_overview.html"><strong aria-hidden="true">4.</strong> Authorization &amp; Processors</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/assumptions.html"><strong aria-hidden="true">4.1.</strong> Assumptions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_contract.html"><strong aria-hidden="true">4.2.</strong> Authorization Contract</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/authorization_instantiation.html"><strong aria-hidden="true">4.2.1.</strong> Instantiation</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_owner_actions.html"><strong aria-hidden="true">4.2.2.</strong> Owner Actions</a></li><li class="chapter-item expanded "><a href="authorizations_processors/authorization_user_actions.html"><strong aria-hidden="true">4.2.3.</strong> User Actions</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/processor_contract.html"><strong aria-hidden="true">4.3.</strong> Processor Contract</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="authorizations_processors/processor.html"><strong aria-hidden="true">4.3.1.</strong> Processor</a></li><li class="chapter-item expanded "><a href="authorizations_processors/lite_processor.html"><strong aria-hidden="true">4.3.2.</strong> Lite Processor</a></li><li class="chapter-item expanded "><a href="authorizations_processors/execution_environment_differences.html"><strong aria-hidden="true">4.3.3.</strong> Execution Environment Differences</a></li></ol></li><li class="chapter-item expanded "><a href="authorizations_processors/callbacks.html"><strong aria-hidden="true">4.4.</strong> Callbacks</a></li><li class="chapter-item expanded "><a href="authorizations_processors/connectors.html"><strong aria-hidden="true">4.5.</strong> Connectors</a></li><li class="chapter-item expanded "><a href="authorizations_processors/encoding.html"><strong aria-hidden="true">4.6.</strong> Encoding</a></li></ol></li><li class="chapter-item expanded "><a href="libraries/_overview.html"><strong aria-hidden="true">5.</strong> Libraries</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="libraries/cosmwasm/_overview.html"><strong aria-hidden="true">5.1.</strong> CosmWasm</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="libraries/cosmwasm/astroport_lper.html"><strong aria-hidden="true">5.1.1.</strong> Astroport LPer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/astroport_withdrawer.html"><strong aria-hidden="true">5.1.2.</strong> Astroport Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/forwarder.html"><strong aria-hidden="true">5.1.3.</strong> Forwarder</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/generic_ibc_transfer.html"><strong aria-hidden="true">5.1.4.</strong> Generic IBC Transfer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/neutron_ibc_transfer.html"><strong aria-hidden="true">5.1.5.</strong> Neutron IBC Transfer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/osmosis_cl_lper.html"><strong aria-hidden="true">5.1.6.</strong> Osmosis CL LPer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/osmosis_cl_withdrawer.html"><strong aria-hidden="true">5.1.7.</strong> Osmosis CL Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/osmosis_gamm_lper.html"><strong aria-hidden="true">5.1.8.</strong> Osmosis GAMM LPer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/osmosis_gamm_withdrawer.html"><strong aria-hidden="true">5.1.9.</strong> Osmosis GAMM Withdrawer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/reverse_splitter.html"><strong aria-hidden="true">5.1.10.</strong> Reverse Splitter</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/splitter.html"><strong aria-hidden="true">5.1.11.</strong> Splitter</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/neutron_ic_querier.html"><strong aria-hidden="true">5.1.12.</strong> Neutron Interchain Querier</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/drop_liquid_staker.html"><strong aria-hidden="true">5.1.13.</strong> Drop Liquid Staker</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/drop_liquid_unstaker.html"><strong aria-hidden="true">5.1.14.</strong> Drop Liquid Unstaker</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/ica_cctp_transfer.html"><strong aria-hidden="true">5.1.15.</strong> ICA CCTP Transfer</a></li><li class="chapter-item expanded "><a href="libraries/cosmwasm/ica_ibc_transfer.html"><strong aria-hidden="true">5.1.16.</strong> ICA IBC Transfer</a></li></ol></li><li class="chapter-item expanded "><a href="libraries/evm/_overview.html"><strong aria-hidden="true">5.2.</strong> EVM</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="libraries/evm/forwarder.html"><strong aria-hidden="true">5.2.1.</strong> Forwarder</a></li><li class="chapter-item expanded "><a href="libraries/evm/cctp_transfer.html"><strong aria-hidden="true">5.2.2.</strong> CCTP Transfer</a></li><li class="chapter-item expanded "><a href="libraries/evm/stargate_transfer.html"><strong aria-hidden="true">5.2.3.</strong> Stargate Transfer</a></li><li class="chapter-item expanded "><a href="libraries/evm/aave_position_manager.html"><strong aria-hidden="true">5.2.4.</strong> AAVE Position Manager</a></li><li class="chapter-item expanded "><a href="libraries/evm/balancer_v2_swap.html"><strong aria-hidden="true">5.2.5.</strong> Balancer V2 Swap</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="middleware/_overview.html"><strong aria-hidden="true">6.</strong> Middleware</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="middleware/broker.html"><strong aria-hidden="true">6.1.</strong> Broker</a></li><li class="chapter-item expanded "><a href="middleware/type_registry.html"><strong aria-hidden="true">6.2.</strong> Type Registry</a></li><li class="chapter-item expanded "><a href="middleware/valence_types.html"><strong aria-hidden="true">6.3.</strong> Valence Types</a></li><li class="chapter-item expanded "><a href="middleware/valence_asserter.html"><strong aria-hidden="true">6.4.</strong> Valence Asserter</a></li></ol></li><li class="chapter-item expanded "><a href="examples/_overview.html"><strong aria-hidden="true">7.</strong> Examples</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="examples/token_swap.html"><strong aria-hidden="true">7.1.</strong> Token Swap</a></li><li class="chapter-item expanded "><a href="examples/crosschain_vaults.html"><strong aria-hidden="true">7.2.</strong> Crosschain Vaults</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="examples/vault_strategist.html"><strong aria-hidden="true">7.2.1.</strong> Vault Strategist</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="testing/_overview.html"><strong aria-hidden="true">8.</strong> Testing</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="testing/setup.html"><strong aria-hidden="true">8.1.</strong> Initial Testing Set Up</a></li><li class="chapter-item expanded "><a href="testing/without_program_manager.html"><strong aria-hidden="true">8.2.</strong> Example without Program Manager</a></li><li class="chapter-item expanded "><a href="testing/with_program_manager.html"><strong aria-hidden="true">8.3.</strong> Example with Program Manager</a></li></ol></li><li class="chapter-item expanded "><a href="security.html"><strong aria-hidden="true">9.</strong> Security</a></li><li class="chapter-item expanded "><a href="deployment/_overview.html"><strong aria-hidden="true">10.</strong> Deployment</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="deployment/local.html"><strong aria-hidden="true">10.1.</strong> Local Deployment</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
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
