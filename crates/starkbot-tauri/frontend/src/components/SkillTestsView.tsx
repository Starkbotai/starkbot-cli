import { useState, useEffect, useMemo, useRef } from "react";
import type { SkillTestInfo, SkillTestRunReport, SkillTestResult, SkillTestStep } from "../types";

interface Props {
  skillTests: SkillTestInfo[];
  skillTestRunning: string | null;
  skillTestReport: SkillTestRunReport | null;
  skillTestPartialResults: SkillTestResult[];
  skillTestCurrentTest: string | null;
  skillTestSteps: Record<string, SkillTestStep[]>;
  onListSkillTests: () => void;
  onSaveSkillTest: (id: string, content: string) => void;
  onDeleteSkillTest: (id: string) => void;
  onRunSkillTest: (id: string) => void;
}

const SKELETON_RON = `(
    name: "New Test Suite",
    tests: [
        (
            id: "test-1",
            name: "Example test",
            prompt: "Read the file Cargo.toml",
            expect_tools: Some(["read_file"]),
            expect_no_error: Some(true),
            retries: Some(1),
        ),
    ],
)`;

export default function SkillTestsView({
  skillTests,
  skillTestRunning,
  skillTestReport,
  skillTestPartialResults,
  skillTestCurrentTest,
  skillTestSteps,
  onListSkillTests,
  onSaveSkillTest,
  onDeleteSkillTest,
  onRunSkillTest,
}: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editorContent, setEditorContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [reports, setReports] = useState<Record<string, SkillTestRunReport>>({});
  const [expandedTraces, setExpandedTraces] = useState<Record<string, boolean>>({});
  const [expandedFinalText, setExpandedFinalText] = useState<Record<string, boolean>>({});
  const pendingSelectRef = useRef<string | null>(null);
  const traceEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    onListSkillTests();
  }, []);

  // Store reports by suite_id as they arrive
  useEffect(() => {
    if (skillTestReport) {
      setReports((prev) => ({ ...prev, [skillTestReport.suite_id]: skillTestReport }));
    }
  }, [skillTestReport]);

  // Auto-select newly created test
  useEffect(() => {
    if (pendingSelectRef.current) {
      const found = skillTests.find((t) => t.id === pendingSelectRef.current);
      if (found) {
        setSelectedId(found.id);
        pendingSelectRef.current = null;
      }
    }
  }, [skillTests]);

  const selected = useMemo(
    () => skillTests.find((t) => t.id === selectedId) ?? null,
    [skillTests, selectedId]
  );

  // Sync editor when selection changes
  useEffect(() => {
    if (selected) {
      setEditorContent(selected.content);
      setDirty(false);
    }
  }, [selected?.id]);

  const switchSelection = (id: string) => {
    if (dirty) {
      if (!confirm("You have unsaved changes. Discard them?")) return;
    }
    setSelectedId(id);
  };

  const handleNew = () => {
    if (dirty && !confirm("You have unsaved changes. Discard them?")) return;
    const slug = `test-${Date.now()}`;
    pendingSelectRef.current = slug;
    onSaveSkillTest(slug, SKELETON_RON);
  };

  const handleSave = () => {
    if (selectedId) {
      onSaveSkillTest(selectedId, editorContent);
      setDirty(false);
    }
  };

  const handleDelete = () => {
    if (selectedId && confirm(`Delete test suite "${selected?.name ?? selectedId}"?`)) {
      onDeleteSkillTest(selectedId);
      setSelectedId(null);
      setDirty(false);
    }
  };

  const handleRun = () => {
    if (selectedId) {
      // Auto-save if dirty before running
      if (dirty) {
        onSaveSkillTest(selectedId, editorContent);
        setDirty(false);
      }
      onRunSkillTest(selectedId);
    }
  };

  // Auto-scroll trace when running test gets new steps
  useEffect(() => {
    if (skillTestCurrentTest && traceEndRef.current) {
      traceEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [skillTestCurrentTest, skillTestSteps]);

  const toggleTrace = (testId: string) => {
    setExpandedTraces((prev) => ({ ...prev, [testId]: !prev[testId] }));
  };

  const toggleFinalText = (testId: string) => {
    setExpandedFinalText((prev) => ({ ...prev, [testId]: !prev[testId] }));
  };

  // Find report for selected suite (from accumulated reports)
  const report = selectedId ? reports[selectedId] ?? null : null;

  return (
    <div className="h-full flex">
      {/* Left panel: suite list */}
      <div className="w-1/4 border-r border-surface-3 flex flex-col">
        <div className="px-3 py-2 border-b border-surface-3 flex items-center justify-between">
          <span className="text-xs text-gray-400 font-medium uppercase tracking-wide">Test Suites</span>
          <button
            onClick={handleNew}
            className="px-2 py-0.5 rounded text-xs bg-accent/20 text-accent hover:bg-accent/30 transition-colors"
          >
            + New
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {skillTests.length === 0 ? (
            <div className="p-4 text-sm text-gray-500">
              No test suites yet. Click "+ New" to create one.
            </div>
          ) : (
            skillTests.map((test) => {
              const testReport = reports[test.id];
              const isRunning = skillTestRunning === test.id;
              return (
                <button
                  key={test.id}
                  onClick={() => switchSelection(test.id)}
                  className={`w-full text-left px-3 py-2.5 border-b border-surface-2 transition-colors ${
                    selectedId === test.id
                      ? "bg-surface-2 border-l-2 border-l-accent"
                      : "hover:bg-surface-1"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-gray-200">{test.name}</span>
                    {isRunning && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-yellow-900/40 text-yellow-400 animate-pulse">
                        running
                      </span>
                    )}
                    {!isRunning && testReport && (
                      <span
                        className={`text-[10px] px-1.5 py-0.5 rounded ${
                          testReport.failed === 0
                            ? "bg-green-900/40 text-green-400"
                            : "bg-red-900/40 text-red-400"
                        }`}
                      >
                        {testReport.failed === 0 ? "passed" : `${testReport.failed} failed`}
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-gray-500 mt-0.5">
                    {test.test_count} test{test.test_count !== 1 ? "s" : ""} &middot; {test.id}
                  </div>
                </button>
              );
            })
          )}
        </div>

        <div className="px-3 py-2 border-t border-surface-3">
          <button
            onClick={onListSkillTests}
            className="px-3 py-1 rounded text-xs bg-surface-2 text-gray-300 hover:bg-surface-3 hover:text-white transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>

      {/* Center panel: RON editor */}
      <div className="w-1/2 flex flex-col border-r border-surface-3">
        {selected ? (
          <>
            <div className="px-3 py-2 border-b border-surface-3 flex items-center gap-2">
              <span className="text-sm font-medium text-gray-200 flex-1">
                {selected.name}
                {dirty && <span className="text-yellow-400 ml-1">*</span>}
              </span>
              <button
                onClick={handleSave}
                disabled={!dirty}
                className="px-3 py-1 rounded text-xs bg-accent/20 text-accent hover:bg-accent/30 transition-colors disabled:opacity-30"
              >
                Save
              </button>
              <button
                onClick={handleRun}
                disabled={!!skillTestRunning}
                className="px-3 py-1 rounded text-xs bg-green-900/30 text-green-400 hover:bg-green-900/50 transition-colors disabled:opacity-30"
              >
                {skillTestRunning === selectedId ? "Running..." : "Run"}
              </button>
              <button
                onClick={handleDelete}
                className="px-3 py-1 rounded text-xs bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors"
              >
                Delete
              </button>
            </div>
            <textarea
              value={editorContent}
              onChange={(e) => {
                setEditorContent(e.target.value);
                setDirty(true);
              }}
              spellCheck={false}
              className="flex-1 w-full p-3 bg-surface-0 text-gray-200 font-mono text-xs resize-none focus:outline-none"
              placeholder="RON test definition..."
            />
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
            Select a test suite to edit
          </div>
        )}
      </div>

      {/* Right panel: run results */}
      <div className="w-1/4 flex flex-col">
        <div className="px-3 py-2 border-b border-surface-3">
          <span className="text-xs text-gray-400 font-medium uppercase tracking-wide">Results</span>
        </div>

        <div className="flex-1 overflow-y-auto p-3">
          {(() => {
            const isRunningThis = skillTestRunning === selectedId;
            // Show partial results while running, final report when done
            const displayResults = isRunningThis ? skillTestPartialResults : report?.results;

            if (!displayResults && !isRunningThis) {
              return (
                <div className="text-gray-500 text-sm">
                  {selectedId ? "Run a test suite to see results" : "Select a test suite first"}
                </div>
              );
            }

            const partialPassed = displayResults?.filter((r) => r.passed).length ?? 0;
            const partialFailed = displayResults?.filter((r) => !r.passed).length ?? 0;

            return (
              <>
                {/* Summary header */}
                <div className="flex items-center gap-2 mb-3">
                  {report && !isRunningThis ? (
                    <>
                      <span className={`text-sm font-medium ${report.failed === 0 ? "text-green-400" : "text-red-400"}`}>
                        {report.passed} passed, {report.failed} failed
                      </span>
                      <span className="text-xs text-gray-500">{(report.duration_ms / 1000).toFixed(1)}s</span>
                    </>
                  ) : (
                    <>
                      <span className="text-sm font-medium text-yellow-400 animate-pulse">Running...</span>
                      <span className="text-xs text-gray-500">
                        {partialPassed + partialFailed} done
                        {partialFailed > 0 && <span className="text-red-400 ml-1">({partialFailed} failed)</span>}
                      </span>
                    </>
                  )}
                </div>

                {/* Individual test results (streamed in as they complete) */}
                {displayResults?.map((r) => {
                  const steps = skillTestSteps[r.test_id] ?? [];
                  const traceExpanded = expandedTraces[r.test_id] ?? false;
                  const finalTextExpanded = expandedFinalText[r.test_id] ?? false;
                  return (
                    <div
                      key={r.test_id}
                      className={`mb-2 p-2 rounded text-xs ${
                        r.passed ? "bg-green-900/20 border border-green-900/30" : "bg-red-900/20 border border-red-900/30"
                      }`}
                    >
                      <div className="flex items-center gap-1.5 mb-1">
                        <span className={r.passed ? "text-green-400" : "text-red-400"}>
                          {r.passed ? "PASS" : "FAIL"}
                        </span>
                        <span className="text-gray-300 font-medium">{r.test_name}</span>
                        <span className="text-gray-600 ml-auto">{(r.duration_ms / 1000).toFixed(1)}s</span>
                      </div>

                      {r.tools_called.length > 0 && (
                        <div className="text-gray-500 mb-0.5">
                          Tools: {r.tools_called.join(", ")}
                        </div>
                      )}

                      {r.error && (
                        <div className="text-red-400 mt-1 break-words">{r.error}</div>
                      )}

                      {/* Step trace (expandable) */}
                      {steps.length > 0 && (
                        <div className="mt-1.5">
                          <button
                            onClick={() => toggleTrace(r.test_id)}
                            className="text-[10px] text-gray-500 hover:text-gray-300 transition-colors"
                          >
                            {traceExpanded ? "Hide" : "Show"} trace ({steps.length} steps)
                          </button>
                          {traceExpanded && (
                            <div className="mt-1 pl-2 border-l border-surface-3 space-y-0.5 max-h-48 overflow-y-auto">
                              {steps.map((s, i) => (
                                <div key={i} className="font-mono text-[10px] leading-relaxed">
                                  {s.kind === "tool_call" && (
                                    <span className="text-blue-400">
                                      {"\u25B6"} {s.name}({s.content.length > 60 ? s.content.slice(0, 60) + "..." : s.content})
                                    </span>
                                  )}
                                  {s.kind === "tool_result" && (
                                    <span className={s.success ? "text-green-400" : "text-red-400"}>
                                      {s.success ? "\u2713" : "\u2717"} {s.name}: {s.content.length > 80 ? s.content.slice(0, 80) + "..." : s.content}
                                    </span>
                                  )}
                                  {s.kind === "thinking" && (
                                    <span className="text-gray-500 italic">
                                      {s.content.length > 80 ? s.content.slice(0, 80) + "..." : s.content}
                                    </span>
                                  )}
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      )}

                      {/* Final text (agent answer) */}
                      {r.final_text && (
                        <div className="mt-1.5">
                          <button
                            onClick={() => toggleFinalText(r.test_id)}
                            className="text-[10px] text-gray-500 hover:text-gray-300 transition-colors"
                          >
                            {finalTextExpanded ? "Hide" : "Show"} answer
                          </button>
                          {finalTextExpanded && (
                            <div className="mt-1 p-1.5 bg-surface-1 rounded text-[10px] text-gray-300 whitespace-pre-wrap break-words max-h-32 overflow-y-auto">
                              {r.final_text.length > 500 ? r.final_text.slice(0, 500) + "..." : r.final_text}
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}

                {/* Currently running test indicator with live steps */}
                {isRunningThis && skillTestCurrentTest && (
                  <div className="mb-2 p-2 rounded text-xs bg-yellow-900/10 border border-yellow-900/20">
                    <div className="flex items-center gap-1.5 mb-1">
                      <span className="text-yellow-400 animate-pulse">RUN</span>
                      <span className="text-gray-400">{skillTestCurrentTest}</span>
                    </div>
                    {/* Live step trace */}
                    {(skillTestSteps[skillTestCurrentTest] ?? []).length > 0 && (
                      <div className="pl-2 border-l border-yellow-900/30 space-y-0.5 max-h-48 overflow-y-auto">
                        {(skillTestSteps[skillTestCurrentTest] ?? []).map((s, i) => (
                          <div key={i} className="font-mono text-[10px] leading-relaxed">
                            {s.kind === "tool_call" && (
                              <span className="text-blue-400">
                                {"\u25B6"} {s.name}({s.content.length > 60 ? s.content.slice(0, 60) + "..." : s.content})
                              </span>
                            )}
                            {s.kind === "tool_result" && (
                              <span className={s.success ? "text-green-400" : "text-red-400"}>
                                {s.success ? "\u2713" : "\u2717"} {s.name}: {s.content.length > 80 ? s.content.slice(0, 80) + "..." : s.content}
                              </span>
                            )}
                            {s.kind === "thinking" && (
                              <span className="text-gray-500 italic">
                                {s.content.length > 80 ? s.content.slice(0, 80) + "..." : s.content}
                              </span>
                            )}
                          </div>
                        ))}
                        <div ref={traceEndRef} />
                      </div>
                    )}
                  </div>
                )}
              </>
            );
          })()}
        </div>
      </div>
    </div>
  );
}
