import { describe, it, expect } from "vitest";
import { resolve } from "node:path";
import { extractMethodsFromCpp } from "../src/index/cppMethods.js";

const referenceRoot = resolve(import.meta.dirname, "..", "..", "..", "reference", "core");

describe("extractMethodsFromCpp", () => {
  it("indexes WorldSession methods in Handler translation units", () => {
    const path = resolve(
      referenceRoot,
      "src/game/Handlers/AuctionHouseHandler.cpp",
    );
    const methods = extractMethodsFromCpp(path, []);

    expect(methods.length).toBeGreaterThan(0);
    expect(methods.every((m) => m.name.startsWith("WorldSession::"))).toBe(true);
    expect(methods.some((m) => m.name === "WorldSession::HandleAuctionHelloOpcode")).toBe(
      true,
    );
  });

  it("indexes multiple classes defined in the same cpp file", () => {
    const path = resolve(referenceRoot, "src/game/AuctionHouse/AuctionHouseMgr.cpp");
    const methods = extractMethodsFromCpp(path, []);
    const names = new Set(methods.map((m) => m.name));

    expect(names.has("AuctionHouseMgr::LoadAuctions")).toBe(true);
    expect(names.has("AuctionHouseObject::AddAuction")).toBe(true);
  });
});
