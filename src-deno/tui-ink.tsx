// Links about ink:
// - [vadimdemedes/ink: 🌈 React for interactive command-line apps](https://github.com/vadimdemedes/ink)
// - [ink - npm](https://www.npmjs.com/package/ink)
// - [Terminal Wordle: Write a Wordle Clone for the Terminal with React Ink](https://spin.atomicobject.com/terminal-wordle-react-ink/)
// - [Deno support · Issue #250 · vadimdemedes/ink](https://github.com/vadimdemedes/ink/issues/250)
//
// Note that there are third part components for ink:
// https://github.com/vadimdemedes/ink#useful-components
//
// Investigate mouse event support:
// https://github.com/zenobi-us/ink-mouse
//
// To use React devtools run the following commands:
// echo '{"nodeModulesDir": "auto"}' > deno.jsonc
// deno install --allow-scripts npm:react-devtools@4.22
// deno run -ERS npm:react-devtools
// $env:DEV="true"
// deno run -E deno_tui_ink.tsx

import "./tui-ink/eager-permissions.ts";
import { ink, path_extname, React } from "./tui-ink/deps.ts";
import { EventEmitter } from "node:events";
import {
  BoundToTerminalSize,
  ConsoleSizeProvider,
} from "./tui-ink/console-size.tsx";
import { useOnMouseClick } from "./tui-ink/mouse.ts";
import { getElementRegion, isIntersecting } from "./tui-ink/position.ts";
import { patchStdinObject, patchStdoutObject } from "./tui-ink/patch-stdio.ts";
import {
  requestPermissionOnMainBuffer,
  switchToSecondaryTerminalBuffer,
} from "./tui-ink/secondary-buffer.ts";
import {
  Button,
  Checkbox,
  DropDown,
  SelectableList,
  SelectableListItem,
  TextArea,
  TextField,
} from "./tui-ink/components.tsx";
import { asOverlay, OverlayProvider } from "./tui-ink/overlay.tsx";

import {
  findFirefoxProfilesDirectory,
  getWasm,
  prepareWasiContextArguments,
} from "./wasi-snapshot-preview1.runner.ts";
import { findDownloadsFolder, writeTextToClipboard } from "./utils.ts";
import {
  ContextOptions as WasmContextOptions,
} from "./wasi-snapshot-preview1.ts";
import { MouseProvider } from "./tui-ink/mouse/context.tsx";
import {
  runWasmCommand,
  WasmCommandResult,
  WasmCommandRunOptions,
} from "./tui-ink/worker/common.ts";
import { WasmCommandWorker } from "./tui-ink/worker/client.ts";

const { useMemo, useRef, useState, useEffect } = React;
const {
  Box,
  Text,
  Spacer,
  useInput,
  useApp,
  useFocusManager,
  render,
} = ink;

export type WindowSelectProps = {
  openWindows: string[];
  closedWindows: string[];
  selectedOpenWindows: number[];
  setSelectedOpenWindows: (value: number[]) => void;
  selectedClosedWindows: number[];
  setSelectedClosedWindows: (value: number[]) => void;
};
export function WindowSelect(props: WindowSelectProps) {
  const {
    openWindows,
    closedWindows,
    selectedOpenWindows,
    setSelectedOpenWindows,
    selectedClosedWindows,
    setSelectedClosedWindows,
  } = props;

  const selectedIds = [
    ...selectedOpenWindows.map((ix) => "open-" + ix),
    ...selectedClosedWindows.map((ix) => "closed-" + ix),
  ];
  function setSelected(ids: string[]) {
    const open: number[] = [];
    const closed: number[] = [];
    for (const id of ids) {
      if (id.startsWith("open-")) {
        const index = parseInt(id.slice("open-".length));
        open.push(index);
      }
      if (id.startsWith("closed-")) {
        const index = parseInt(id.slice("closed-".length));
        closed.push(index);
      }
    }
    setSelectedOpenWindows(open);
    setSelectedClosedWindows(closed);
  }

  return (
    <SelectableList
      outerBoxProps={{
        paddingLeft: 3,
        paddingRight: 3,
        flexGrow: 1,
      }}
      innerBoxProps={{
        flexDirection: "column",
      }}
      scrollMargin={2}
      selectedIds={selectedIds}
      setSelectedIds={setSelected}
    >
      <Box flexDirection="column" flexShrink={0}>
        <Text bold underline>Open Windows</Text>
        {openWindows.map((name, ix) => (
          <SelectableListItem key={ix} id={"open-" + ix} name={name} />
        ))}
      </Box>
      <Box flexDirection="column" paddingTop={1} flexShrink={0}>
        <Text bold underline>Closed Windows</Text>
        {closedWindows.map((name, ix) => (
          <SelectableListItem key={ix} id={"closed-" + ix} name={name} />
        ))}
      </Box>
    </SelectableList>
  );
}

export type InputAreaProps = {
  inputPath: string;
  setInputPath: (value: string) => void;
  loadedPath: string;
  onOpenWizard: () => void;
  onLoadInput: (inputPath: string) => void;
};

/** Handles selecting and loading an input file. */
export function InputArea(props: InputAreaProps) {
  const { inputPath, setInputPath, loadedPath, onOpenWizard, onLoadInput } =
    props;

  return (
    <Box width="100%" flexShrink={0} flexDirection="column">
      <Box flexDirection="row" width="100%" alignItems="center">
        <Box flexShrink={0} marginRight={1}>
          <Text>Path to sessionstore file:</Text>
        </Box>
        <TextField value={inputPath} onChange={setInputPath} />
        <Button label="Wizard" focusId="wizard-button" onClick={onOpenWizard} />
      </Box>
      <Box flexDirection="row" width="100%" alignItems="center">
        <Box flexShrink={0} marginRight={1}>
          <Text>Current data was loaded from:</Text>
        </Box>
        <TextField value={loadedPath} onChange={null} />
        <Button
          label="Load new data"
          onClick={() => onLoadInput(inputPath)}
        />
      </Box>
    </Box>
  );
}

export interface OutputOptions {
  outputPath: string;
  createFolder: boolean;
  overwriteFile: boolean;
  outputFormat: string;
}

export interface OutputFormatInfo {
  name: string;
  alias_for: null | string;
  is_supported: boolean;
  description: string;
  file_extension: string;
}

export type OutputAreaProps = {
  onCopyToClipboard: () => void;
  onGenerateOutput: (options: OutputOptions) => void;
  outputFormats: OutputFormatInfo[];
};

/** Handles selecting output location, configuring the output format and
 * actually generating the output file. */
export function OutputArea(props: OutputAreaProps) {
  const [outputPath, setOutputPath] = useState(() =>
    (findDownloadsFolder() ?? ".") + "/firefox-links"
  );
  const [createFolder, setCreateFolder] = useState(false);
  const [overwriteFile, setOverwriteFile] = useState(false);
  const [outputFormat, setOutputFormat] = useState("pdf");

  return (
    <Box width="100%" flexShrink={0} flexDirection="column">
      <Box flexDirection="row" width="100%" alignItems="center">
        <Box flexShrink={0} marginRight={1}>
          <Text>File path to write links to:</Text>
        </Box>
        <TextField value={outputPath} onChange={setOutputPath} />
      </Box>
      <Box flexDirection="row" width="100%" alignItems="center">
        <Checkbox
          checked={createFolder}
          setChecked={setCreateFolder}
          label="Create folder if it doesn't exist"
        />
        <Checkbox
          checked={overwriteFile}
          setChecked={setOverwriteFile}
          marginLeft={4}
          label="Overwrite file if it already exists"
        />
      </Box>
      <Box flexDirection="row" width="100%" alignItems="center">
        <Button
          label="Copy links to clipboard"
          onClick={props.onCopyToClipboard}
        />
        <Spacer />
        <Box marginRight={4} flexDirection="row">
          <Box alignSelf="center" marginRight={1}>
            <Text bold>Output format:</Text>
          </Box>
          <DropDown selectedId={outputFormat} setSelectedId={setOutputFormat}>
            {props.outputFormats.filter((format) => format.is_supported).map(
              (format, ix) => {
                return (
                  <SelectableListItem
                    key={ix}
                    id={format.name}
                    name={format.name}
                  />
                );
              },
            )}
          </DropDown>
        </Box>
        <Button
          label="Save links to file"
          onClick={() =>
            props.onGenerateOutput({
              outputPath,
              createFolder,
              overwriteFile,
              outputFormat,
            })}
        />
      </Box>
    </Box>
  );
}

export type StatusBarRef = ReturnType<typeof createStatusBarRef>;
function createStatusBarRef() {
  return {
    _id: 1,
    setStatus(_value: string, _previousId?: number | null): number | null {
      return null;
    },
    setProgressReport(
      _value: string,
      _previousId?: number | null,
    ): number | null {
      return null;
    },
    /** Don't reset progress counter but do update the shown message. */
    updateProgressReport(
      _value: string,
      _previousId?: number | null,
    ): number | null {
      return null;
    },
  };
}
export type StatusBarProps = {
  refStatus: React.Ref<StatusBarRef>;
};
export function StatusBar(props: StatusBarProps) {
  const [status, setStatus] = useState("");
  const [lastProgress, setLastProgress] = useState<number | null>(null);
  const refStatus = useRef<StatusBarRef>(null);
  if (refStatus.current === null) {
    refStatus.current = createStatusBarRef();
  }
  refStatus.current.setStatus = function (value, previousId) {
    if (Boolean(previousId) && previousId !== this._id) return null;
    this._id++;
    setStatus(value);
    setLastProgress(null);
    return this._id;
  };
  refStatus.current.setProgressReport = function (value, previousId) {
    if (Boolean(previousId) && previousId !== this._id) return null;
    this._id++;
    setStatus(value);
    setLastProgress(Date.now());
    return this._id;
  };
  refStatus.current.updateProgressReport = function (value, previousId) {
    if (Boolean(previousId) && previousId !== this._id) return null;
    this._id++;
    setStatus(value);
    return this._id;
  };

  if (props.refStatus) {
    if (typeof props.refStatus === "function") {
      props.refStatus(refStatus.current);
    } else {
      props.refStatus.current = refStatus.current;
    }
  }

  const [, setUpdate] = useState(0);
  useEffect(() => {
    if (lastProgress === null) return;
    const intervalId = setInterval(() => {
      setUpdate((c) => c + 1);
    }, 1000 / 60);
    return () => clearInterval(intervalId);
  }, [lastProgress === null]);

  return (
    <Box flexShrink={0} alignItems="center">
      <Box flexShrink={0} marginRight={1}>
        <Text>Status:</Text>
      </Box>
      <TextField
        value={status + (lastProgress === null
          ? ""
          : ` (${(Date.now() - lastProgress) / 1000} seconds)`)}
        onChange={null}
      />
    </Box>
  );
}

export type WizardProps = {
  refOverlay?: React.Ref<ink.DOMElement>;
  refStatus: React.RefObject<StatusBarRef | null>;
  onWizardClose: (selectedPath: string | null) => void;
};
export function Wizard(props: WizardProps) {
  const { onWizardClose, refStatus, refOverlay } = props;

  const refBackground = useRef<ink.DOMElement>(null);
  const refWizard = useRef<ink.DOMElement>(null);
  useOnMouseClick(refBackground, (event, mousePos) => {
    if (
      // Click on background:
      event &&
      // Not click on wizard:
      !isIntersecting({
        region: getElementRegion(refWizard.current),
        position: mousePos,
      })
    ) {
      onWizardClose(null);
    }
  });
  useInput((_input, key) => {
    if (!key.ctrl && !key.shift && key.escape) {
      onWizardClose(null);
    }
  });

  const profilesDir = useMemo<string | null>(() => {
    if (!requestPermissionOnMainBuffer({ name: "env" })) {
      Promise.resolve().then(() => {
        onWizardClose(null);
        refStatus.current?.setStatus(
          `Error: permission denied to read environment variables, so can't find Firefox profiles directory`,
        );
      });
      return null;
    }
    return findFirefoxProfilesDirectory(Deno.env.toObject());
  }, []);

  const profiles = useMemo<string[]>(() => {
    if (!profilesDir) return [];
    const profiles: string[] = [];
    if (!requestPermissionOnMainBuffer({ name: "read", path: profilesDir })) {
      Promise.resolve().then(() => {
        onWizardClose(null);
        refStatus.current?.setStatus(
          `Error: permission denied to read firefox profiles directory, so can't determine possible profile names`,
        );
      });
      return [];
    }
    for (const entry of Deno.readDirSync(profilesDir)) {
      if (!entry.isDirectory) continue;
      profiles.push(entry.name);
    }
    //profiles.push(...Array.from({ length: 40 }, () => fake.food.fruit()));
    return profiles;
  }, [profilesDir]);

  function setSelected(ids: string[]) {
    if (ids.length !== 1) return;
    if (!profilesDir) return onWizardClose(null);
    const profilePath = profilesDir + "/" + ids[0];
    const potentialFiles = [
      "sessionstore.jsonlz4",
      "sessionstore-backups/recovery.jsonlz4",
    ];

    for (const file of potentialFiles) {
      const filePath = profilePath + "/" + file;
      try {
        if (!requestPermissionOnMainBuffer({ name: "read", path: filePath })) {
          refStatus.current?.setStatus(
            `Error: permission denied to check if file existed at \"${filePath}\"`,
          );
          onWizardClose(null);
          return;
        }
        if (Deno.statSync(filePath).isFile) {
          onWizardClose(filePath);
          return;
        }
      } catch (error) {
        if (error instanceof Deno.errors.NotFound) {
          // ignore
        } else {
          refStatus.current?.setStatus(
            `Error: failed to check if file existed at \"${filePath}\": ${error}`,
          );
          onWizardClose(null);
          return;
        }
      }
    }

    refStatus.current?.setStatus(
      `Error: no sessionstore files found at the default locations for the Firefox profile at \"${profilePath}\"`,
    );
    onWizardClose(null);
  }

  return (
    <Box
      ref={refBackground}
      height="100%"
      width="100%"
      alignItems="center"
      flexDirection="column"
    >
      <Spacer />
      <Box
        ref={(element) => {
          refWizard.current = element;
          if (refOverlay) {
            if (typeof refOverlay === "function") {
              refOverlay(element);
            } else {
              refOverlay.current = element;
            }
          }
        }}
        minHeight={10}
        marginY={1}
        paddingY={1}
        paddingX={6}
        flexDirection="column"
        alignItems="center"
        borderStyle="round"
      >
        <Text bold underline>Firefox Profiles</Text>
        <SelectableList
          selectedIds={[]}
          setSelectedIds={setSelected}
          autoFocus
          outerBoxProps={{ marginTop: 1, borderStyle: "bold" }}
          innerBoxProps={{ flexDirection: "column" }}
        >
          {(profiles || []).map((profile, index) => (
            <SelectableListItem
              key={index}
              id={String(profile)}
              name={profile}
            />
          ))}
        </SelectableList>
      </Box>
      <Spacer />
    </Box>
  );
}

/** Show the {@link Wizard} as an overlay. */
export const WizardOverlay = asOverlay(Wizard);

export interface AppProps {
  runCommand: (options: WasmCommandRunOptions) => Promise<WasmCommandResult>;
  outputFormats: OutputFormatInfo[];
}
interface FirefoxState {
  data: Uint8Array<ArrayBuffer> | null;
}
export function App(props: AppProps) {
  const app = useApp();
  useInput((input, key) => {
    if (key.ctrl && (input === "d" || input === "D")) {
      app.exit();
    }
  });

  const refStatus = useRef<StatusBarRef>(null);
  const [wizardOpen, setWizardOpen] = useState(false);

  const [inputPath, setInputPath] = useState("");
  const [loadedPath, setLoadedPath] = useState("");

  const [openGroups, setOpenGroups] = useState<string[]>([]);
  const [closedGroups, setClosedGroups] = useState<string[]>([]);
  const [selectedOpenGroups, setSelectedOpenGroups] = useState<number[]>([]);
  const [selectedClosedGroups, setSelectedClosedGroups] = useState<number[]>(
    [],
  );
  const focusManager = useFocusManager();

  const [previewText, setPreviewText] = useState("");

  const firefoxState = useMemo<FirefoxState>(() => ({
    data: null,
  }), []);

  useEffect(() => {
    generatePreview();
  }, [openGroups, closedGroups, selectedOpenGroups, selectedClosedGroups]);

  async function onLoadInput(wantedInputPath: string) {
    firefoxState.data = null;
    setOpenGroups([]);
    setClosedGroups([]);
    setSelectedOpenGroups([]);
    setSelectedClosedGroups([]);
    setLoadedPath(wantedInputPath);

    if (
      !requestPermissionOnMainBuffer({ name: "read", path: wantedInputPath })
    ) {
      refStatus.current?.setStatus(
        `Error: permission denied to read input file at: \"${wantedInputPath}\"`,
      );
      return;
    }
    let statusId = refStatus.current?.setProgressReport(`Reading input file`);
    let data: Uint8Array<ArrayBuffer>;
    try {
      data = Deno.readFileSync(wantedInputPath);
    } catch (error) {
      refStatus.current?.setStatus(
        `Error: failed to read input file at: \"${wantedInputPath}\": ${error}`,
        statusId,
      );
      return;
    }
    firefoxState.data = data;

    statusId = refStatus.current?.setProgressReport(`Finding tab groups`);
    const outputGroups = await props.runCommand({
      args: [
        "get-groups",
        // Input:
        "--stdin",
        "--compressed",
        // Output:
        "--stdout",
        "--closed-windows",
        "--json",
      ],
      stdin: data,
    }).catch(() => null);
    if (!outputGroups) {
      refStatus.current?.setStatus(
        `Error: canceled when finding tab groups`,
        statusId,
      );
      return;
    }
    const groups: { name: string; tab_count: number; is_closed: boolean }[] =
      JSON.parse(outputGroups.stdoutString);
    setOpenGroups(
      groups.filter((g) => !g.is_closed).map((g) =>
        g.name + ` (${g.tab_count})`
      ),
    );
    setClosedGroups(
      groups.filter((g) => g.is_closed).map((g) =>
        g.name + ` (${g.tab_count})`
      ),
    );
    refStatus.current?.setStatus("Successfully found tab groups", statusId);
  }

  function argsToSelectTabGroups(): string[] {
    if (selectedClosedGroups.length === 0 && selectedOpenGroups.length === 0) {
      return [];
    } else {
      return [
        "--closed-windows",
        "--tab-group-indexes=" +
        [
          ...selectedOpenGroups,
          ...selectedClosedGroups.map((ix) => openGroups.length + ix),
        ],
      ];
    }
  }

  async function generatePreview() {
    if (!firefoxState.data) {
      setPreviewText("");
      return;
    }
    const statusId = refStatus.current?.setProgressReport(`Generating preview`);
    const output = await props.runCommand({
      args: [
        "tabs-to-links",
        // Input:
        "--stdin",
        "--compressed",
        // Output:
        "--stdout",
        "--format=text",
        "--tree-data=sidebery,tst",
        ...argsToSelectTabGroups(),
      ],
      stdin: firefoxState.data,
    }).catch(() => null);
    if (!output) {
      refStatus.current?.setStatus("preview canceled", statusId);
      return;
    }
    refStatus.current?.setStatus(
      `Preview successfully generated (exit code: ${output.exitCode})`,
      statusId,
    );
    setPreviewText(output.stdoutString);
  }

  function onCopyToClipboard() {
    writeTextToClipboard(previewText, { sync: true });
    refStatus.current?.setStatus("Copied data to clipboard");
  }

  async function onGenerateOutput(options: OutputOptions) {
    if (!firefoxState.data) {
      refStatus.current?.setStatus(
        "Error: can't generate output without first loading input data",
      );
      return;
    }

    const [formatInfo] = props.outputFormats.filter((format) =>
      format.name === options.outputFormat
    );

    const actualExt = path_extname.extname(options.outputPath); // ".pdf" or empty if no extension

    let wantedExt;
    // No extension so guess from format.
    if (options.outputFormat.includes("pdf")) {
      wantedExt = ".pdf";
    } else if (options.outputFormat.includes("rtf")) {
      wantedExt = ".rtf";
    } else if (options.outputFormat.includes("markdown")) {
      wantedExt = ".md";
    } else if (options.outputFormat.includes("text")) {
      wantedExt = ".txt";
    } else if (options.outputFormat.includes("typst")) {
      wantedExt = ".typ";
    } else if (options.outputFormat.includes("html")) {
      wantedExt = ".html";
    }
    if (formatInfo.file_extension) {
      // Actually we can get the extension directly from the WASM program:
      wantedExt = "." + formatInfo.file_extension;
    }
    if (actualExt === "") {
      options.outputPath += wantedExt;
    }

    const wasAllowed = requestPermissionOnMainBuffer({
      name: "write",
      path: options.outputPath,
    });
    if (!wasAllowed) {
      refStatus.current?.setStatus(
        `Error: denied permission for writing a file to "${options.outputPath}"`,
      );
      return;
    }

    refStatus.current?.setProgressReport("Generating output");
    const extraArgs: string[] = [];
    if (
      options.outputFormat.includes("html") ||
      // All pdf converters except typst uses html:
      (options.outputFormat.includes("pdf") &&
        !options.outputFormat.includes("typst"))
    ) {
      // HTML specific options:
      // Hard to see long links in HTML so indent them:
      extraArgs.push("--indent-all-links");
    }
    const output = await props.runCommand({
      args: [
        "tabs-to-links",
        // Input:
        "--stdin",
        "--compressed",
        // Output:
        "--stdout",
        `--format=${options.outputFormat}`,
        "--tree-data=sidebery,tst",
        "--page-breaks",
        ...extraArgs,
        ...argsToSelectTabGroups(),
      ],
      stdin: firefoxState.data,
    }).catch(() => null);
    if (!output) {
      refStatus.current?.setProgressReport(
        `Error: canceled when generating output file`,
      );
      return;
    }
    if (output.exitCode !== 0) {
      refStatus.current?.setStatus(
        `Error: failed to generate output (exit code: ${output.exitCode})`,
      );
      // TODO: open error popup instead of reusing preview area.
      setPreviewText(output.stderrString);
      return;
    }
    refStatus.current?.setStatus(
      `Successfully generated output file (exit code: ${output.exitCode})`,
    );

    if (options.createFolder) {
      const lastIndex = Math.max(
        options.outputPath.lastIndexOf("/"),
        options.outputPath.lastIndexOf("\\"),
      );
      const folderPath = options.outputPath.slice(0, lastIndex);
      if (lastIndex >= 0) {
        if (
          requestPermissionOnMainBuffer({ name: "write", path: folderPath })
        ) {
          try {
            Deno.mkdirSync(folderPath, { recursive: true });
          } catch (error) {
            refStatus.current?.setStatus(
              `Error: failed to create output folder at \"${folderPath}\": ${error}`,
            );
          }
        }
      }
    }

    try {
      Deno.writeFileSync(options.outputPath, output.stdout, {
        createNew: !options.overwriteFile,
      });
      refStatus.current?.setStatus(
        `Successfully written output to file at \"${options.outputPath}\"`,
      );
    } catch (error) {
      refStatus.current?.setStatus(
        `Error: failed to write output to file at \"${options.outputPath}\": ${error}`,
      );
    }
  }

  return (
    <ConsoleSizeProvider>
      <MouseProvider>
        <BoundToTerminalSize>
          <OverlayProvider>
            <Box
              flexDirection="row"
              width="100%"
              height="100%"
              padding={0}
              margin={0}
            >
              <Box
                height="100%"
                minWidth={32}
                flexDirection="column"
                overflowX="hidden"
              >
                <WindowSelect
                  openWindows={openGroups}
                  closedWindows={closedGroups}
                  selectedOpenWindows={selectedOpenGroups}
                  setSelectedOpenWindows={setSelectedOpenGroups}
                  selectedClosedWindows={selectedClosedGroups}
                  setSelectedClosedWindows={setSelectedClosedGroups}
                />
                <Box flexShrink={0}>
                  <Button label="Quit (Ctrl+D)" onClick={() => app.exit()} />
                </Box>
              </Box>
              <Box
                flexDirection="column"
                marginLeft={1}
                width="100%"
                height="100%"
              >
                <InputArea
                  inputPath={inputPath}
                  setInputPath={setInputPath}
                  loadedPath={loadedPath}
                  onOpenWizard={() => setWizardOpen(true)}
                  onLoadInput={onLoadInput}
                />
                <Box flexShrink={0}>
                  <Text>Tabs as links:</Text>
                </Box>
                <TextArea
                  outerBoxProps={{ flexGrow: 1 }}
                  value={previewText}
                  onChange={null}
                />
                <OutputArea
                  outputFormats={props.outputFormats}
                  onCopyToClipboard={onCopyToClipboard}
                  onGenerateOutput={onGenerateOutput}
                />
                <StatusBar refStatus={refStatus} />
              </Box>
            </Box>
            <WizardOverlay
              isOverlayOpen={wizardOpen}
              refStatus={refStatus}
              onWizardClose={(inputPath) => {
                setWizardOpen(false);
                focusManager.focus("wizard-button");
                if (inputPath) {
                  setInputPath(inputPath);
                  onLoadInput(inputPath);
                }
              }}
            />
          </OverlayProvider>
        </BoundToTerminalSize>
      </MouseProvider>
    </ConsoleSizeProvider>
  );
}

export async function singleThreadedRunCommand(
  wasmData: Uint8Array,
  wasmContextOptions: WasmContextOptions,
): Promise<AppProps["runCommand"]> {
  const wasmModule = await WebAssembly.compile(wasmData);
  return async (options) => {
    return await runWasmCommand({
      wasmContextOptions,
      wasmModule,
      ...options,
    });
  };
}

export async function main(wasmSource?: string, multiThreaded: boolean = true) {
  if (wasmSource === undefined) {
    if (Deno.args.length !== 1) {
      throw new Error(
        `Expected a single argument to "tui.ink.tsx" script which should be the location where the WASM file is located (You can specify "IMPORT" to download it from GitHub).`,
      );
    }
    wasmSource = Deno.args[0];
  }

  const wasmData = await getWasm(Deno.args[0]);
  const wasmContextOptions = await prepareWasiContextArguments([], {
    preopenFirefoxProfilesDir: false,
  });

  let worker: WasmCommandWorker | null = null;
  let runCommand: AppProps["runCommand"];
  if (multiThreaded) {
    worker = WasmCommandWorker.create({
      wasmData,
      wasmContextOptions,
    });
    const w = worker;
    runCommand = (options) => w.run(options);
  } else {
    runCommand = await singleThreadedRunCommand(wasmData, wasmContextOptions);
  }

  const outputFormats: OutputFormatInfo[] = JSON.parse(
    (await runCommand({
      args: ["tabs-to-links-formats", "--json"],
    })).stdoutString,
  );

  // Prevent https://www.npmjs.com/package/signal-exit from intercepting Ctrl+C signal and then requesting --allow-run permission.
  // Deno.addSignalListener("SIGINT", () => { _renderer.unmount(); Deno.exit(0); });

  patchStdinObject();
  patchStdoutObject();
  switchToSecondaryTerminalBuffer();

  // Disable EventEmitter warnings: https://docs.deno.com/api/node/events/~/EventEmitter.defaultMaxListeners
  EventEmitter.defaultMaxListeners = 0;

  render(
    <App runCommand={runCommand} outputFormats={outputFormats} />,
    { exitOnCtrlC: false },
  ).waitUntilExit().finally(() => {
    worker?.[Symbol.dispose]();
  });
  globalThis.addEventListener("unload", () => worker?.[Symbol.dispose]());
}
if (import.meta.main) {
  await main();
}
