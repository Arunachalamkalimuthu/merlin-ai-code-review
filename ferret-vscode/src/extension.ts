/**
 * Merlin VS Code Extension
 *
 * Provides AI code review, explanation, test generation, and more
 * by calling the merlin CLI binary as a subprocess.
 */

import * as vscode from "vscode";
import { execFile } from "child_process";
import * as path from "path";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ReviewComment {
  file: string;
  line: number;
  severity: string;
  category: string;
  title: string;
  body: string;
  suggestion?: string;
}

// ── Global state ──────────────────────────────────────────────────────────────

let outputChannel: vscode.OutputChannel;
let statusBar: vscode.StatusBarItem;
let diagnosticCollection: vscode.DiagnosticCollection;

// ── Activation ────────────────────────────────────────────────────────────────

export function activate(context: vscode.ExtensionContext): void {
  outputChannel = vscode.window.createOutputChannel("Merlin");
  diagnosticCollection =
    vscode.languages.createDiagnosticCollection("merlin");

  statusBar = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100
  );
  statusBar.text = "$(search) Merlin";
  statusBar.tooltip = "Click to run Merlin review";
  statusBar.command = "merlin.review";

  const cfg = vscode.workspace.getConfiguration("merlin");
  if (cfg.get<boolean>("showStatusBar", true)) {
    statusBar.show();
  }

  // Register commands
  const commands: [string, () => Promise<void>][] = [
    ["merlin.review", cmdReview],
    ["merlin.reviewSelection", cmdReviewSelection],
    ["merlin.explain", cmdExplain],
    ["merlin.improve", cmdImprove],
    ["merlin.security", cmdSecurity],
    ["merlin.test", cmdTest],
    ["merlin.docs", cmdDocs],
    ["merlin.ask", cmdAsk],
    ["merlin.configure", cmdConfigure],
  ];

  for (const [id, handler] of commands) {
    context.subscriptions.push(
      vscode.commands.registerCommand(id, handler)
    );
  }

  // Auto-review on save (if enabled)
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (doc) => {
      const cfg = vscode.workspace.getConfiguration("merlin");
      if (cfg.get<boolean>("autoReviewOnSave", false)) {
        await reviewFile(doc.uri.fsPath, doc.getText());
      }
    })
  );

  context.subscriptions.push(outputChannel, statusBar, diagnosticCollection);

  outputChannel.appendLine("Merlin extension activated 🦡");
}

export function deactivate(): void {
  diagnosticCollection.clear();
}

// ── Command handlers ──────────────────────────────────────────────────────────

async function cmdReview(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage("Merlin: Open a file to review.");
    return;
  }
  await reviewFile(editor.document.uri.fsPath, editor.document.getText());
}

async function cmdReviewSelection(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.selection.isEmpty) {
    vscode.window.showWarningMessage(
      "Merlin: Select some code first."
    );
    return;
  }
  const selected = editor.document.getText(editor.selection);
  const filePath = editor.document.uri.fsPath;
  await runMerlinGenerate(
    "Review the following code snippet for bugs, security issues, and improvements.",
    `File: ${path.basename(filePath)}\n\n${selected}`,
    "Merlin: Selection Review"
  );
}

async function cmdExplain(): Promise<void> {
  const code = await getSelectedOrFullFile();
  if (!code) return;
  await runMerlinGenerate(
    "Explain this code clearly and concisely for a developer who is new to this codebase. " +
      "Describe what it does, how it works, and any important patterns or gotchas.",
    code,
    "Merlin: Code Explanation"
  );
}

async function cmdImprove(): Promise<void> {
  const code = await getSelectedOrFullFile();
  if (!code) return;
  await runMerlinGenerate(
    "Suggest concrete improvements for this code: performance, readability, correctness, and best practices. " +
      "Provide specific, actionable suggestions with code examples.",
    code,
    "Merlin: Improvement Suggestions"
  );
}

async function cmdSecurity(): Promise<void> {
  const code = await getSelectedOrFullFile();
  if (!code) return;
  await runMerlinGenerate(
    "Perform a security review of this code. Look for OWASP Top 10 vulnerabilities, " +
      "injection flaws, hardcoded secrets, insecure API usage, and business logic flaws. " +
      "Report findings in order of severity.",
    code,
    "Merlin: Security Scan"
  );
}

async function cmdTest(): Promise<void> {
  const code = await getSelectedOrFullFile();
  if (!code) return;
  const editor = vscode.window.activeTextEditor;
  const lang = editor?.document.languageId ?? "unknown";
  await runMerlinGenerate(
    `Generate comprehensive unit tests for this ${lang} code. ` +
      "Cover happy paths, edge cases, error conditions, and boundary values. " +
      "Use the standard test framework for this language.",
    code,
    "Merlin: Generated Tests"
  );
}

async function cmdDocs(): Promise<void> {
  const code = await getSelectedOrFullFile();
  if (!code) return;
  const editor = vscode.window.activeTextEditor;
  const lang = editor?.document.languageId ?? "unknown";
  await runMerlinGenerate(
    `Generate comprehensive documentation for this ${lang} code. ` +
      "Include module/class/function docstrings, parameter descriptions, return values, " +
      "and usage examples. Use the idiomatic doc format for the language.",
    code,
    "Merlin: Generated Docs"
  );
}

async function cmdAsk(): Promise<void> {
  const question = await vscode.window.showInputBox({
    prompt: "Ask Merlin anything about your code...",
    placeHolder: "e.g. Is this function thread-safe? What does this algorithm do?",
  });
  if (!question) return;

  const code = await getSelectedOrFullFile();
  if (!code) return;

  await runMerlinGenerate(
    "You are a helpful senior software engineer. Answer the developer's question about their code clearly and accurately.",
    `Question: ${question}\n\nCode:\n${code}`,
    "Merlin: Answer"
  );
}

async function cmdConfigure(): Promise<void> {
  vscode.commands.executeCommand(
    "workbench.action.openSettings",
    "merlin"
  );
}

// ── Core review logic ─────────────────────────────────────────────────────────

/**
 * Write a temp diff file and run `merlin review --diff <file> --output json`,
 * then parse the output and display as VS Code diagnostics.
 */
async function reviewFile(filePath: string, content: string): Promise<void> {
  setStatus("$(loading~spin) Reviewing...");
  outputChannel.appendLine(`\nReviewing: ${filePath}`);

  try {
    // Build a pseudo unified diff from the current file content
    const pseudoDiff = buildPseudoDiff(filePath, content);

    const result = await runMerlinCli(
      ["review", "--diff", "-", "--output", "json"],
      pseudoDiff
    );

    const comments: ReviewComment[] = parseJsonOutput(result);
    showDiagnostics(filePath, comments);

    if (comments.length === 0) {
      vscode.window.showInformationMessage(
        "Merlin: No issues found ✅"
      );
    } else {
      vscode.window
        .showWarningMessage(
          `Merlin found ${comments.length} issue(s).`,
          "Show Output"
        )
        .then((action) => {
          if (action === "Show Output") outputChannel.show();
        });
    }

    setStatus(`$(search) Merlin (${comments.length} issues)`);
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    outputChannel.appendLine(`Error: ${msg}`);
    vscode.window.showErrorMessage(`Merlin error: ${msg}`);
    setStatus("$(search) Merlin");
  }
}

/**
 * Run a freeform AI generation via `merlin ask` or a direct generate call.
 * Shows output in a new editor tab.
 */
async function runMerlinGenerate(
  system: string,
  user: string,
  title: string
): Promise<void> {
  setStatus("$(loading~spin) Running...");
  outputChannel.appendLine(`\n[${title}]`);

  try {
    // Use merlin's CLI stdin mode: merlin ask --system "..." --user "-"
    const result = await runMerlinCli(
      ["ask", "--system", system, "--stdin"],
      user
    );

    // Show in a virtual Markdown document
    await showMarkdownPanel(title, result);
    setStatus("$(search) Merlin");
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    outputChannel.appendLine(`Error: ${msg}`);
    vscode.window.showErrorMessage(`Merlin error: ${msg}`);
    setStatus("$(search) Merlin");
  }
}

// ── Merlin CLI runner ─────────────────────────────────────────────────────────

function merlinBinary(): string {
  const cfg = vscode.workspace.getConfiguration("merlin");
  return cfg.get<string>("binaryPath", "merlin");
}

function workspaceRoot(): string {
  return vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath ?? process.cwd();
}

function runMerlinCli(args: string[], stdin?: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const bin = merlinBinary();
    const cwd = workspaceRoot();
    const env = { ...process.env };

    outputChannel.appendLine(`$ ${bin} ${args.join(" ")}`);

    const proc = execFile(bin, args, { cwd, env }, (err, stdout, stderr) => {
      if (stderr) outputChannel.appendLine(`[stderr] ${stderr}`);
      if (err) {
        reject(new Error(stderr || err.message));
        return;
      }
      resolve(stdout);
    });

    if (stdin && proc.stdin) {
      proc.stdin.write(stdin);
      proc.stdin.end();
    }
  });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async function getSelectedOrFullFile(): Promise<string | undefined> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage("Merlin: Open a file first.");
    return undefined;
  }
  if (!editor.selection.isEmpty) {
    return editor.document.getText(editor.selection);
  }
  return editor.document.getText();
}

function parseJsonOutput(raw: string): ReviewComment[] {
  try {
    const trimmed = raw
      .trim()
      .replace(/^```json\s*/m, "")
      .replace(/^```\s*/m, "")
      .replace(/```\s*$/m, "");
    return JSON.parse(trimmed) as ReviewComment[];
  } catch {
    return [];
  }
}

function showDiagnostics(
  filePath: string,
  comments: ReviewComment[]
): void {
  diagnosticCollection.clear();

  const uri = vscode.Uri.file(filePath);
  const diagnostics: vscode.Diagnostic[] = comments.map((c) => {
    const line = Math.max(0, c.line - 1);
    const range = new vscode.Range(line, 0, line, 999);
    const diag = new vscode.Diagnostic(
      range,
      `[${c.severity.toUpperCase()}] ${c.title}\n${c.body}`,
      severityToVscode(c.severity)
    );
    diag.source = "Merlin";
    diag.code = c.category;
    return diag;
  });

  diagnosticCollection.set(uri, diagnostics);
}

function severityToVscode(
  sev: string
): vscode.DiagnosticSeverity {
  switch (sev.toLowerCase()) {
    case "critical":
    case "high":
      return vscode.DiagnosticSeverity.Error;
    case "medium":
      return vscode.DiagnosticSeverity.Warning;
    case "low":
      return vscode.DiagnosticSeverity.Information;
    default:
      return vscode.DiagnosticSeverity.Hint;
  }
}

function buildPseudoDiff(filePath: string, content: string): string {
  const rel = path.relative(workspaceRoot(), filePath);
  const lines = content.split("\n").map((l, i) => `+${l}`).join("\n");
  return `--- /dev/null\n+++ b/${rel}\n@@ -0,0 +1,${content.split("\n").length} @@\n${lines}\n`;
}

async function showMarkdownPanel(
  title: string,
  content: string
): Promise<void> {
  const panel = vscode.window.createWebviewPanel(
    "merlinResult",
    title,
    vscode.ViewColumn.Beside,
    { enableScripts: false }
  );

  // Convert basic markdown to HTML
  const escaped = content
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  panel.webview.html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <style>
    body { font-family: var(--vscode-font-family); color: var(--vscode-editor-foreground);
           background: var(--vscode-editor-background); padding: 1rem 1.5rem; line-height: 1.6; }
    pre { background: var(--vscode-textCodeBlock-background); padding: 1rem; border-radius: 4px;
          overflow-x: auto; font-size: 0.9em; }
    code { font-family: var(--vscode-editor-font-family); }
    h1,h2,h3 { color: var(--vscode-textLink-foreground); }
  </style>
</head>
<body>
  <h2>${title}</h2>
  <pre><code>${escaped}</code></pre>
</body>
</html>`;
}

function setStatus(text: string): void {
  statusBar.text = text;
}
