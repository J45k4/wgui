import { Document, Text } from "https://deno.land/x/deno_dom@v0.1.22-alpha/deno-dom-wasm.ts";
import { getPathItem } from "./path.ts";
import { assertEquals, assert } from "https://deno.land/std@0.167.0/testing/asserts.ts";

Deno.test("getPathItem", () => {
    const document = new Document()

    const root = document.createElement("div");
    
    const child = document.createElement("div");
    
    root.appendChild(child);
    
    const text = document.createTextNode("text");
    child.appendChild(text);

    console.log(root)
    console.log(child)
    
    const el = getPathItem([0, 0], root as any);

    assert(el instanceof HTMLDivElement);
    
    console.log("el", el);

    // if (!el) {
    //     throw new Error("el is null");
    // }
});
