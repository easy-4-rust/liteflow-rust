#!/usr/bin/env node

/**
 * 使用两个 CodeGraph 索引审计 Java 公共方法到 Rust snake_case 方法的静态覆盖率。
 *
 * 该工具只证明“对应文件中存在同名公共入口”，不能证明参数、返回值、异常、
 * 并发行为或外部服务语义已经一致。对象映射以 docs/对象级对照表.md 为准。
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { basename, isAbsolute, join, resolve } from "node:path";
import assert from "node:assert/strict";

function isPublicRustTraitDeclaration(sourceLine, traitName) {
  return new RegExp(
    `^\\s*pub\\s+(?:unsafe\\s+)?trait\\s+${traitName}(?:\\s|<|:)`,
  ).test(sourceLine ?? "");
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isPublicRustObjectDeclaration(source, objectName) {
  const escapedObjectName = escapeRegExp(objectName);
  return new RegExp(
    `^\\s*pub\\s+(?:struct|enum)\\s+${escapedObjectName}(?:\\s|<|\\{|\\(|;)`,
    "m",
  ).test(source);
}

function implementsTrait(source, objectName, traitName) {
  const escapedObjectName = escapeRegExp(objectName);
  const escapedTraitName = escapeRegExp(traitName);
  return new RegExp(
    `impl(?:\\s*<[^>{}]*>)?\\s+${escapedTraitName}(?:\\s*<[^>{}]*>)?\\s+for\\s+${escapedObjectName}\\b`,
    "m",
  ).test(source);
}

function isDirectJavaMethod(qualifiedName, className) {
  const ownerMarker = `::${className}::`;
  const ownerIndex = qualifiedName.indexOf(ownerMarker);
  if (ownerIndex < 0) {
    return false;
  }
  const ownerTail = qualifiedName.slice(ownerIndex + ownerMarker.length);
  return !ownerTail.includes("::") && !ownerTail.includes("::<");
}

function rustTraitParents(declaration, traitName) {
  const escapedTraitName = escapeRegExp(traitName);
  const match = declaration.match(
    new RegExp(`\\btrait\\s+${escapedTraitName}(?:\\s*<[^>{}]*>)?\\s*:\\s*([^\\{]+)`),
  );
  if (!match) {
    return [];
  }
  return match[1]
    .split("+")
    .map((parent) => parent.trim())
    .filter((parent) => parent.length > 0 && !parent.startsWith("'"))
    .map((parent) => parent.replace(/<.*$/, "").split("::").at(-1).trim());
}

if (process.argv.includes("--self-test")) {
  assert.equal(isPublicRustTraitDeclaration("pub trait Condition: Executable {", "Condition"), true);
  assert.equal(isPublicRustTraitDeclaration("pub unsafe trait Guard<T> {", "Guard"), true);
  assert.equal(isPublicRustTraitDeclaration("pub(crate) trait Internal {", "Internal"), false);
  assert.equal(isPublicRustTraitDeclaration("trait Private {", "Private"), false);
  assert.equal(isPublicRustObjectDeclaration("pub struct ThenOperator;", "ThenOperator"), true);
  assert.equal(isPublicRustObjectDeclaration("pub(crate) struct Hidden;", "Hidden"), false);
  assert.equal(implementsTrait("impl BaseOperator for ThenOperator {", "ThenOperator", "BaseOperator"), true);
  assert.equal(implementsTrait("impl Other for ThenOperator {", "ThenOperator", "BaseOperator"), false);
  assert.equal(
    isDirectJavaMethod("com.yomahub.liteflow::ForOperator::build", "ForOperator"),
    true,
  );
  assert.equal(
    isDirectJavaMethod(
      "com.yomahub.liteflow::ForOperator::build::<NodeForComponent$anon@36>::processFor",
      "ForOperator",
    ),
    false,
  );
  assert.equal(
    isDirectJavaMethod(
      "com.yomahub.liteflow::RuleParsePluginUtil::ChainDto::ChainDto",
      "RuleParsePluginUtil",
    ),
    false,
  );
  assert.deepEqual(
    rustTraitParents("pub trait Condition: Executable + Send + Sync {", "Condition"),
    ["Executable", "Send", "Sync"],
  );
  assert.deepEqual(rustTraitParents("pub trait Standalone {", "Standalone"), []);
  console.log("audit_java_method_parity self-test: ok");
  process.exit(0);
}

const positionalArguments = process.argv.slice(2).filter((argument) => !argument.startsWith("--"));
const summaryOnly = process.argv.includes("--summary");
const rustRoot = resolve(positionalArguments[0] ?? process.cwd());
const javaRoot = resolve(
  positionalArguments[1] ?? join(rustRoot, "../../workspace-github/liteflow"),
);
const mappingPath = join(rustRoot, "docs/对象级对照表.md");
const javaDb = join(javaRoot, ".codegraph/codegraph.db");
const rustDb = join(rustRoot, ".codegraph/codegraph.db");

for (const requiredPath of [mappingPath, javaDb, rustDb]) {
  if (!existsSync(requiredPath)) {
    throw new Error(`缺少审计输入: ${requiredPath}`);
  }
}

function query(database, sql) {
  const output = execFileSync("sqlite3", ["-json", database, sql], {
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  }).trim();
  return output.length === 0 ? [] : JSON.parse(output);
}

function toSnakeCase(name) {
  return name
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2")
    .replace(/([a-z\d])([A-Z])/g, "$1_$2")
    .replace(/[-\s]+/g, "_")
    .toLowerCase();
}

function javaClassNameFromFile(filePath) {
  return basename(filePath, ".java");
}

function rustTargetPath(rawTarget) {
  const normalized = rawTarget.replaceAll("\\", "/");
  if (isAbsolute(normalized)) {
    return normalized.slice(rustRoot.length + 1);
  }
  if (
    normalized.startsWith("liteflow-core/") ||
    normalized.startsWith("liteflow-derive/") ||
    normalized.startsWith("liteflow-vernal/") ||
    normalized.startsWith("liteflow-agent/")
  ) {
    return normalized;
  }
  return `liteflow-core/src/${normalized}`;
}

const mappingRows = readFileSync(mappingPath, "utf8")
  .split(/\r?\n/)
  .map((line) => {
    const match = line.match(/^\|\s*([^|`]+?)\s*\|\s*`([^`]+\.rs)`\s*\|/);
    if (!match) {
      return null;
    }
    const javaObject = match[1]
      .replace(/（.*?）/g, "")
      .replace(/\s+/g, "")
      .trim();
    if (
      javaObject.length === 0 ||
      javaObject.includes("/") ||
      javaObject.includes("{") ||
      javaObject.includes("...")
    ) {
      return null;
    }
    return {
      javaObject,
      rustFile: rustTargetPath(match[2]),
    };
  })
  .filter(Boolean);

const javaMethods = query(
  javaDb,
  `SELECT name, qualified_name AS qualifiedName, file_path AS filePath
   FROM nodes
   WHERE kind = 'method'
     AND visibility = 'public'
     AND file_path LIKE 'liteflow-core/src/main/java/%'`,
);
const rustTraits = query(
  rustDb,
  `SELECT name, file_path AS filePath, start_line AS startLine
   FROM nodes
   WHERE kind = 'trait'`,
);
const publicRustTraitNodes = rustTraits
  .map((traitNode) => {
    const sourceLines = readFileSync(join(rustRoot, traitNode.filePath), "utf8").split(/\r?\n/);
    const sourceLine = sourceLines.at(traitNode.startLine - 1);
    const declaration = sourceLines
      .slice(traitNode.startLine - 1, traitNode.startLine + 8)
      .join(" ")
      .split("{", 1)[0];
    return { ...traitNode, sourceLine, declaration };
  })
  .filter((traitNode) => isPublicRustTraitDeclaration(traitNode.sourceLine, traitNode.name));
const publicRustTraits = new Set(
  publicRustTraitNodes.map((traitNode) => `${traitNode.filePath}\0${traitNode.name}`),
);
const publicRustTraitParents = new Map(
  publicRustTraitNodes.map((traitNode) => [
    traitNode.name,
    rustTraitParents(traitNode.declaration, traitNode.name),
  ]),
);
const allRustCallables = query(
  rustDb,
  `SELECT name, qualified_name AS qualifiedName, file_path AS filePath,
          visibility
   FROM nodes
   WHERE kind IN ('method', 'function')
  `,
);
const publicTraitMethods = new Map();
for (const callable of allRustCallables) {
  const owner = callable.qualifiedName.includes("::")
    ? callable.qualifiedName.slice(0, callable.qualifiedName.indexOf("::"))
    : "";
  if (!publicRustTraits.has(`${callable.filePath}\0${owner}`)) {
    continue;
  }
  const methods = publicTraitMethods.get(owner) ?? new Set();
  methods.add(callable.name.replace(/^r#/, ""));
  publicTraitMethods.set(owner, methods);
}

// Rust 子 trait 会把父 trait 的公开方法作为同一对象的可调用 API 暴露；把这层
// 继承闭包纳入审计，避免把 `Condition: Executable` 的 execute 误报成缺口。
let inheritedTraitMethodAdded = true;
while (inheritedTraitMethodAdded) {
  inheritedTraitMethodAdded = false;
  for (const [traitName, parentNames] of publicRustTraitParents) {
    const methods = publicTraitMethods.get(traitName) ?? new Set();
    const previousSize = methods.size;
    for (const parentName of parentNames) {
      for (const method of publicTraitMethods.get(parentName) ?? []) {
        methods.add(method);
      }
    }
    if (methods.size > previousSize) {
      inheritedTraitMethodAdded = true;
    }
    if (methods.size > 0) {
      publicTraitMethods.set(traitName, methods);
    }
  }
}

const rustCallables = allRustCallables.filter((callable) => {
  if (callable.visibility === "public") {
    return true;
  }
  const owner = callable.qualifiedName.includes("::")
    ? callable.qualifiedName.slice(0, callable.qualifiedName.indexOf("::"))
    : "";
  // Rust trait 内的方法天然继承 trait 的公开性，语法上不能再写 `pub fn`。
  // CodeGraph 当前把这些方法标成 private，因此必须结合 `pub trait` 源码校正。
  return publicRustTraits.has(`${callable.filePath}\0${owner}`);
});

const javaByClass = new Map();
for (const method of javaMethods) {
  const className = javaClassNameFromFile(method.filePath);
  if (method.name === className || !isDirectJavaMethod(method.qualifiedName, className)) {
    continue;
  }
  const key = `${className}\0${method.filePath}`;
  const methods = javaByClass.get(key) ?? new Set();
  methods.add(method.name);
  javaByClass.set(key, methods);
}

const rustByFile = new Map();
for (const callable of rustCallables) {
  const methods = rustByFile.get(callable.filePath) ?? new Set();
  // Rust 关键字方法必须使用原始标识符（例如 FlowEvent.Builder#type →
  // `pub fn r#type`）；对外方法名仍是 `type`，审计时应去掉语法前缀。
  methods.add(callable.name.replace(/^r#/, ""));
  rustByFile.set(callable.filePath, methods);
}

for (const traitNode of publicRustTraitNodes) {
  const methods = rustByFile.get(traitNode.filePath) ?? new Set();
  for (const method of publicTraitMethods.get(traitNode.name) ?? []) {
    methods.add(method);
  }
  if (methods.size > 0) {
    rustByFile.set(traitNode.filePath, methods);
  }
}

// CodeGraph 当前不为零大小 struct 建立类型节点，并把 trait impl 中的方法标记为
// private。公开对象实现公开 trait 时，这些方法仍是其真实公开 API；从源码补齐
// 继承/实现方法，避免要求每个对象再写一层无意义的固有方法代理。
for (const mapping of mappingRows) {
  const absoluteRustFile = join(rustRoot, mapping.rustFile);
  if (!existsSync(absoluteRustFile)) {
    continue;
  }
  const source = readFileSync(absoluteRustFile, "utf8");
  if (!isPublicRustObjectDeclaration(source, mapping.javaObject)) {
    continue;
  }
  const methods = rustByFile.get(mapping.rustFile) ?? new Set();
  for (const [traitName, traitMethods] of publicTraitMethods) {
    if (!implementsTrait(source, mapping.javaObject, traitName)) {
      continue;
    }
    for (const method of traitMethods) {
      methods.add(method);
    }
  }
  if (methods.size > 0) {
    rustByFile.set(mapping.rustFile, methods);
  }
}

const rows = [];
const ambiguousObjects = [];
const missingRustFiles = [];

for (const mapping of mappingRows) {
  const candidates = [...javaByClass.entries()].filter(([key]) =>
    key.startsWith(`${mapping.javaObject}\0`),
  );
  if (candidates.length === 0) {
    continue;
  }
  if (candidates.length > 1) {
    ambiguousObjects.push({
      javaObject: mapping.javaObject,
      candidates: candidates.map(([key]) => key.split("\0")[1]),
    });
    continue;
  }

  const [[key, methods]] = candidates;
  const javaFile = key.split("\0")[1];
  const rustMethods = rustByFile.get(mapping.rustFile);
  if (!rustMethods) {
    missingRustFiles.push(mapping);
    continue;
  }

  for (const javaMethod of methods) {
    const rustMethod = toSnakeCase(javaMethod);
    rows.push({
      javaObject: mapping.javaObject,
      javaFile,
      rustFile: mapping.rustFile,
      javaMethod,
      rustMethod,
      matched: rustMethods.has(rustMethod),
    });
  }
}

const matched = rows.filter((row) => row.matched);
const missing = rows.filter((row) => !row.matched);
const objectCount = new Set(rows.map((row) => row.javaObject)).size;
const completeObjects = new Set(
  [...new Set(rows.map((row) => row.javaObject))].filter((javaObject) =>
    rows.filter((row) => row.javaObject === javaObject).every((row) => row.matched),
  ),
);

console.log("# Java → Rust 公共方法静态覆盖审计");
console.log();
console.log(`- Java 基线: ${javaRoot}`);
console.log(`- Rust 工作区: ${rustRoot}`);
console.log(`- 已纳入对象: ${objectCount}`);
console.log(`- 完全命中对象: ${completeObjects.size}/${objectCount}`);
console.log(`- 公共方法名命中: ${matched.length}/${rows.length}`);
console.log(`- 待核验方法名: ${missing.length}`);
console.log(`- 映射歧义对象: ${ambiguousObjects.length}`);
console.log(`- 未找到 Rust 目标文件: ${missingRustFiles.length}`);
console.log();
console.log(
  "> 说明：此审计只比较公开方法名，不代表签名、重载、异常和运行时语义已经对齐。",
);

if (missing.length > 0 && summaryOnly) {
  const missingByObject = new Map();
  for (const row of missing) {
    const methods = missingByObject.get(row.javaObject) ?? [];
    methods.push(row.rustMethod);
    missingByObject.set(row.javaObject, methods);
  }
  console.log();
  console.log("## 缺口最多的对象（前 20）");
  console.log();
  console.log("| Java 对象 | 未命中数量 | 期望 Rust 方法示例 |");
  console.log("|---|---:|---|");
  for (const [javaObject, methods] of [...missingByObject.entries()]
    .sort((left, right) => right[1].length - left[1].length)
    .slice(0, 20)) {
    console.log(
      `| ${javaObject} | ${methods.length} | ${methods
        .slice(0, 5)
        .map((method) => `\`${method}\``)
        .join("、")} |`,
    );
  }
}

if (missing.length > 0 && !summaryOnly) {
  console.log();
  console.log("## 待核验方法");
  console.log();
  console.log("| Java 对象 | Java 方法 | 期望 Rust 方法 | Rust 文件 |");
  console.log("|---|---|---|---|");
  for (const row of missing) {
    console.log(
      `| ${row.javaObject} | \`${row.javaMethod}\` | \`${row.rustMethod}\` | \`${row.rustFile}\` |`,
    );
  }
}

if (ambiguousObjects.length > 0 && !summaryOnly) {
  console.log();
  console.log("## Java 同名对象歧义");
  console.log();
  for (const item of ambiguousObjects) {
    console.log(`- ${item.javaObject}: ${item.candidates.join(", ")}`);
  }
}

if (missingRustFiles.length > 0 && !summaryOnly) {
  console.log();
  console.log("## 未进入方法审计的 Rust 文件映射");
  console.log();
  for (const item of missingRustFiles) {
    console.log(`- ${item.javaObject}: ${item.rustFile}`);
  }
}
