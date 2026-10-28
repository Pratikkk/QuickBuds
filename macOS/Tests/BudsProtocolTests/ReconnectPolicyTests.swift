import XCTest
@testable import BudsProtocol

final class ReconnectPolicyTests: XCTestCase {
    func testFailuresBackOffAndStayBounded() {
        var policy = ReconnectPolicy()
        XCTAssertEqual(policy.delay, 2)
        let expected = [5, 10, 20, 30, 30, 30]
        for delay in expected {
            policy.failed()
            XCTAssertEqual(policy.delay, delay)
        }
        for _ in 0..<100 { policy.failed() }
        XCTAssertEqual(policy.delay, 30)
    }

    func testSuccessOrExplicitRetryResetsPacing() {
        var policy = ReconnectPolicy()
        for _ in 0..<4 { policy.failed() }
        policy.reset()
        XCTAssertEqual(policy.delay, 2)
        policy.failed()
        XCTAssertEqual(policy.delay, 5)
    }
}
