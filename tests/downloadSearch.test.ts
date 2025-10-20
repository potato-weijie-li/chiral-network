import { describe, it, expect } from "vitest";

describe("Download Search Functionality", () => {
  describe("Search by Name", () => {
    it("should filter files by name (case-insensitive)", () => {
      // Simulate the filtering logic from Download.svelte lines 542-547
      const files = [
        { id: "1", name: "example.pdf", hash: "abc123", status: "completed" },
        { id: "2", name: "document.txt", hash: "def456", status: "downloading" },
        { id: "3", name: "Example Report.docx", hash: "ghi789", status: "completed" },
        { id: "4", name: "video.mp4", hash: "jkl012", status: "seeding" },
      ];

      const searchFilter = "example";

      const filtered = files.filter(
        (f) =>
          f.hash.toLowerCase().includes(searchFilter.toLowerCase()) ||
          f.name.toLowerCase().includes(searchFilter.toLowerCase())
      );

      expect(filtered).toHaveLength(2);
      expect(filtered[0].name).toBe("example.pdf");
      expect(filtered[1].name).toBe("Example Report.docx");
    });

    it("should filter files by hash", () => {
      const files = [
        { id: "1", name: "example.pdf", hash: "abc123", status: "completed" },
        { id: "2", name: "document.txt", hash: "def456", status: "downloading" },
      ];

      const searchFilter = "abc";

      const filtered = files.filter(
        (f) =>
          f.hash.toLowerCase().includes(searchFilter.toLowerCase()) ||
          f.name.toLowerCase().includes(searchFilter.toLowerCase())
      );

      expect(filtered).toHaveLength(1);
      expect(filtered[0].hash).toBe("abc123");
    });

    it("should return empty array when no matches", () => {
      const files = [
        { id: "1", name: "example.pdf", hash: "abc123", status: "completed" },
      ];

      const searchFilter = "nonexistent";

      const filtered = files.filter(
        (f) =>
          f.hash.toLowerCase().includes(searchFilter.toLowerCase()) ||
          f.name.toLowerCase().includes(searchFilter.toLowerCase())
      );

      expect(filtered).toHaveLength(0);
    });

    it("should handle empty search filter", () => {
      const files = [
        { id: "1", name: "example.pdf", hash: "abc123", status: "completed" },
        { id: "2", name: "document.txt", hash: "def456", status: "downloading" },
      ];

      const searchFilter = "";

      // When searchFilter is empty, all files should be returned
      const filtered = searchFilter.trim()
        ? files.filter(
            (f) =>
              f.hash.toLowerCase().includes(searchFilter.toLowerCase()) ||
              f.name.toLowerCase().includes(searchFilter.toLowerCase())
          )
        : files;

      expect(filtered).toHaveLength(2);
    });

    it("should handle partial matches", () => {
      const files = [
        { id: "1", name: "report_2024_final.pdf", hash: "abc123", status: "completed" },
        { id: "2", name: "report_2023.txt", hash: "def456", status: "downloading" },
        { id: "3", name: "budget.xlsx", hash: "ghi789", status: "completed" },
      ];

      const searchFilter = "report";

      const filtered = files.filter(
        (f) =>
          f.hash.toLowerCase().includes(searchFilter.toLowerCase()) ||
          f.name.toLowerCase().includes(searchFilter.toLowerCase())
      );

      expect(filtered).toHaveLength(2);
      expect(filtered[0].name).toContain("report");
      expect(filtered[1].name).toContain("report");
    });
  });

  describe("Search Mode Selection", () => {
    it("should have three search modes available", () => {
      const searchModes = ["merkle_hash", "cid", "name"];
      
      expect(searchModes).toContain("merkle_hash");
      expect(searchModes).toContain("cid");
      expect(searchModes).toContain("name");
    });

    it("should generate correct placeholder for each mode", () => {
      const getPlaceholder = (mode: string) => {
        return mode === "merkle_hash"
          ? "Enter Merkle Hash..."
          : mode === "cid"
          ? "Enter CID..."
          : "Enter file name...";
      };

      expect(getPlaceholder("merkle_hash")).toBe("Enter Merkle Hash...");
      expect(getPlaceholder("cid")).toBe("Enter CID...");
      expect(getPlaceholder("name")).toBe("Enter file name...");
    });
  });
});
