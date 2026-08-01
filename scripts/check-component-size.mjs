import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import ts from "typescript";

const MAX_COMPONENT_LINES = 200;

function getLineNumber(sourceFile, position) {
  return sourceFile.getLineAndCharacterOfPosition(position).line;
}

function isPureComment(line, inBlockComment) {
  const trimmed = line.trim();
  if (!trimmed) {
    return { comment: false, inBlockComment };
  }
  if (inBlockComment) {
    return {
      comment: true,
      inBlockComment: !trimmed.includes("*/"),
    };
  }
  if (trimmed.startsWith("//")) {
    return { comment: true, inBlockComment: false };
  }
  if (trimmed.startsWith("/*")) {
    return {
      comment: true,
      inBlockComment: !trimmed.includes("*/"),
    };
  }
  return { comment: false, inBlockComment: false };
}

function isComponentDeclaration(node) {
  if (ts.isFunctionDeclaration(node)) {
    return Boolean(node.name && /^[A-Z]/.test(node.name.text));
  }
  return ts.isVariableStatement(node)
    && node.declarationList.declarations.some((declaration) =>
      ts.isIdentifier(declaration.name)
      && /^[A-Z]/.test(declaration.name.text)
      && declaration.initializer
      && (ts.isArrowFunction(declaration.initializer)
        || ts.isFunctionExpression(declaration.initializer)),
    );
}

function getComponentName(node) {
  if (ts.isFunctionDeclaration(node)) {
    return node.name?.text ?? "default";
  }
  const declaration = node.declarationList.declarations.find((item) =>
    ts.isIdentifier(item.name) && /^[A-Z]/.test(item.name.text),
  );
  return declaration && ts.isIdentifier(declaration.name)
    ? declaration.name.text
    : "anonymous";
}

function hasException(sourceFile, node) {
  const startLine = getLineNumber(sourceFile, node.getStart(sourceFile));
  const lines = sourceFile.text.split(/\r?\n/);
  for (let index = startLine - 1; index >= Math.max(0, startLine - 3); index -= 1) {
    if (/component-size-exception:\s*\S+/.test(lines[index] ?? "")) {
      return true;
    }
    if ((lines[index] ?? "").trim() && !lines[index].trim().startsWith("//")) {
      break;
    }
  }
  return false;
}

export function analyzeSource(fileName, sourceText) {
  const sourceFile = ts.createSourceFile(
    fileName,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const lines = sourceText.split(/\r?\n/);
  const importLines = new Set();
  for (const statement of sourceFile.statements) {
    if (ts.isImportDeclaration(statement)) {
      const start = getLineNumber(sourceFile, statement.getStart(sourceFile));
      const end = getLineNumber(sourceFile, statement.end);
      for (let line = start; line <= end; line += 1) {
        importLines.add(line);
      }
    }
  }

  return sourceFile.statements
    .filter(isComponentDeclaration)
    .map((node) => {
      const start = getLineNumber(sourceFile, node.getStart(sourceFile));
      const end = getLineNumber(sourceFile, node.end);
      let inBlockComment = false;
      let effectiveLines = 0;
      for (let line = start; line <= end; line += 1) {
        if (importLines.has(line)) {
          continue;
        }
        const result = isPureComment(lines[line] ?? "", inBlockComment);
        inBlockComment = result.inBlockComment;
        if ((lines[line] ?? "").trim() && !result.comment) {
          effectiveLines += 1;
        }
      }
      return {
        exception: hasException(sourceFile, node),
        lines: effectiveLines,
        name: getComponentName(node),
        startLine: start + 1,
      };
    });
}

function walk(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      return walk(entryPath);
    }
    return entry.name.endsWith(".tsx") && !entry.name.includes(".test.")
      ? [entryPath]
      : [];
  });
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(import.meta.filename)) {
  const failures = walk(path.resolve("src")).flatMap((fileName) =>
    analyzeSource(fileName, fs.readFileSync(fileName, "utf8"))
      .filter((component) => component.lines > MAX_COMPONENT_LINES && !component.exception)
      .map((component) => ({ ...component, fileName })),
  );
  if (failures.length > 0) {
    for (const failure of failures) {
      console.error(
        `${path.relative(process.cwd(), failure.fileName)}:${failure.startLine} ${failure.name} has ${failure.lines} effective lines (max ${MAX_COMPONENT_LINES}).`,
      );
    }
    process.exitCode = 1;
  }
}
