// JSR package (MIT) (this file is using this library, but current version doesn't work on Windows):
// https://github.com/Im-Beast/deno_tui/tree/v3-rewrite
// https://jsr.io/@tui/tui@3.0.0-dev.10
//
// Investigate NPM package (MIT) (fails to run in deno with cryptic errors. likely missing node compatibility):
// https://www.npmjs.com/package/blessed

import { crayon } from "jsr:@crayon/crayon@4.0.0-alpha.4";
import { getColorSupport } from "jsr:@crayon/color-support@2.1.0";

import { tui } from "jsr:@tui/tui@3.0.0-dev.10";
import {
  Button,
  createBlockButton,
  ScrollView,
  Spinner,
  Suspense,
  TextBox,
} from "jsr:@tui/tui@3.0.0-dev.10/components";
import { VerticalBlock } from "jsr:/@tui/nice@^0.6.1/layout";

import { HorizontalBlock } from "jsr:/@tui/nice@^0.6.1/layout/horizontal";
import { Style } from "jsr:/@tui/nice@^0.6.1/style_block";

crayon.colorSupport = await getColorSupport();

// Code from (not exported though): jsr:@tui/tui@3.0.0-dev.10/components/colors.ts
export const colors = {
  background: 0x000000,
  backgroundHigher: 0x363636,
  backgroundHighest: 0x545454,

  textLowest: 0x808080,
  textLower: 0xC0C0C0,
  text: 0xFFFFFF,
  textHigher: 0xFFFFFF,
  textHighest: 0xFFFFFF,

  accent: 0x0060FF,
  accentHigher: 0x0020AF,
  accentHighest: 0x0080CF,
};

function demoUi() {
  const block = new Style({
    string: crayon.bgRed,
    padding: { y: 1, right: 2 },
  });
  const edge = block.derive({
    border: {
      left: true,
      style: crayon.bgRed.bold,
      type: "thick",
    },
  });

  const BlockButton = createBlockButton((id, state) => {
    if (state === "active") {
      return new HorizontalBlock(
        {},
        edge.create("", {
          string: crayon.bgBlue,
          border: { style: crayon.bgBlue },
        }),
        block.create(id, { string: crayon.bgBlue }),
      );
    }

    if (state === "hover") {
      return new HorizontalBlock(
        {},
        edge.create("", {
          string: crayon.bgYellow,
          border: { style: crayon.bgYellow },
        }),
        block.create(id, { string: crayon.bgYellow }),
      );
    }

    return new HorizontalBlock(
      {},
      edge.create(""),
      block.create(id),
    );
  });

  tui.render(() =>
    new VerticalBlock(
      {
        width: "100%",
        height: "100%",
        string: crayon.bgHex(colors.background),
      },
      // Buttons
      Button("abc"),
      Button("xyz"),
      // TextBoxes
      TextBox("Textbox"),
      TextBox("Multiline textbox", {
        multiline: true,
      }),
      BlockButton("BlockButton"),
      new HorizontalBlock(
        { height: "20%", width: "100%", gap: 4 },
        // ScrollView
        ScrollView(
          { id: "scroll-view", height: 5, width: 30 },
          Button("abc"),
          Button("def"),
          Button("ghi"),
          Button("jkl"),
          Button("zxc"),
          Button("vbn"),
          Button("mlp"),
        ),
        // Suspense
        new HorizontalBlock(
          { gap: 4 },
          Suspense("3s", async () => {
            await new Promise((r) => setTimeout(r, 3000));
            return Button("Loaded after 3s!");
          }, Spinner("Waiting 3 seconds")),
          Suspense(
            "5s",
            new Promise((resolve) => {
              setTimeout(() => {
                resolve(Button("Loaded after 5s!"));
              }, 5000);
            }),
            Spinner("Waiting 5 seconds"),
          ),
        ),
      ),
    )
  );
}

demoUi();
