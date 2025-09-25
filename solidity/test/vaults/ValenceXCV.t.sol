import {Test, console} from "forge-std/src/Test.sol";
import {ValenceXCV} from "../../src/vaults/ValenceXCV.sol";

// run with: forge test --match-path test/vaults/ValenceXCV.t.sol -vvv

contract ValenceXCVTest is Test {
    ValenceXCV internal vault;

    function setUp() public {
        console.log("Setting up ValenceXCVTest");
        vault = new ValenceXCV();
    }

    function testSetUpVault() public {
        console.log("set up complete");
    }
}
