import { describe, it, expect } from "vitest";
import { cleanPortRustCode } from "../src/verify/portOutput.js";

describe("cleanPortRustCode", () => {
  it("removes PORT trace comments but keeps TODOs", () => {
    const input = `pub fn foo() {
    // PORT: src/game/Foo.cpp:L10
    // TODO: confirm null handling
    // check if item exists
    let x = 1;
}`;

    expect(cleanPortRustCode(input)).toBe(`pub fn foo() {
    // TODO: confirm null handling
    // check if item exists
    let x = 1;
}`);
  });

  it("removes C++ doc comments", () => {
    const input = `/// C++ AuctionHouseMgr::GetAItem
fn get_a_item() {}
/// Loads auctions from DB
fn load() {}`;

    expect(cleanPortRustCode(input)).toBe(`fn get_a_item() {}
/// Loads auctions from DB
fn load() {}`);
  });
});
