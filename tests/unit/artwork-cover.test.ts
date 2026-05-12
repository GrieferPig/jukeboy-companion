import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import ArtworkCover from "../../src/components/ArtworkCover.vue";

const testArtworkDataUrl = "data:image/svg+xml;charset=UTF-8,%3Csvg%20xmlns%3D%22http%3A//www.w3.org/2000/svg%22%20viewBox%3D%220%200%2016%2016%22%3E%3Crect%20width%3D%2216%22%20height%3D%2216%22%20fill%3D%22%23000%22/%3E%3C/svg%3E";

describe("ArtworkCover", () => {
    it("shows a neutral tile when no artwork source is available", () => {
        const wrapper = mount(ArtworkCover, {
            props: {
                title: "No artwork",
                src: null,
                height: 64,
            },
        });

        expect(wrapper.find("img").exists()).toBe(false);
        expect(wrapper.find(".artwork-cover__fallback").classes()).not.toContain("artwork-cover__fallback--hidden");
    });

    it("keeps the fallback visible until artwork finishes loading", async () => {
        const wrapper = mount(ArtworkCover, {
            props: {
                title: "Loaded artwork",
                src: testArtworkDataUrl,
                height: 64,
            },
        });

        expect(wrapper.find("img").exists()).toBe(true);
        expect(wrapper.find(".artwork-cover__fallback").classes()).not.toContain("artwork-cover__fallback--hidden");

        await wrapper.find("img").trigger("load");

        expect(wrapper.find("img").classes()).toContain("artwork-cover__image--ready");
        expect(wrapper.find(".artwork-cover__fallback").classes()).toContain("artwork-cover__fallback--hidden");
    });

    it("falls back to the neutral tile when the artwork source errors", async () => {
        const wrapper = mount(ArtworkCover, {
            props: {
                title: "Broken artwork",
                src: "data:image/unknown;base64,AAAA",
                height: 64,
            },
        });

        await wrapper.find("img").trigger("error");

        expect(wrapper.find("img").exists()).toBe(false);
        expect(wrapper.find(".artwork-cover__fallback").classes()).not.toContain("artwork-cover__fallback--hidden");
    });
});