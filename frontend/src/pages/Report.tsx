import { useCallback, useMemo, useRef, useState } from 'react';

type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';

interface Finding {
  id?: string;
  package?: string;
  name?: string;
  version?: string;
  severity: Severity;
  title?: string;
  description?: string;
  cve?: string;
  url?: string;
  [key: string]: unknown;
}

interface Summary {
  total?: number;
  critical?: number;
  high?: number;
  medium?: number;
  low?: number;
  info?: number;
  [key: string]: unknown;
}

interface ReportData {
  summary: Summary;
  findings: Finding[];
  [key: string]: unknown;
}

const SEVERITIES: Severity[] = ['critical', 'high', 'medium', 'low', 'info'];

const SEVERITY_STYLES: Record<Severity, string> = {
  critical: 'bg-red-100 text-red-800 border-red-300',
  high: 'bg-orange-100 text-orange-800 border-orange-300',
  medium: 'bg-yellow-100 text-yellow-800 border-yellow-300',
  low: 'bg-blue-100 text-blue-800 border-blue-300',
  info: 'bg-gray-100 text-gray-800 border-gray-300',
};

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isSeverity(value: unknown): value is Severity {
  return typeof value === 'string' && (SEVERITIES as string[]).includes(value);
}

function validateReport(data: unknown): { ok: true; report: ReportData } | { ok: false; error: string } {
  if (!isObject(data)) {
    return { ok: false, error: 'The file does not contain a JSON object.' };
  }
  if (!isObject(data.summary)) {
    return { ok: false, error: 'Missing or invalid "summary" object.' };
  }
  if (!Array.isArray(data.findings)) {
    return { ok: false, error: 'Missing or invalid "findings" array.' };
  }
  for (let i = 0; i < data.findings.length; i += 1) {
    const finding = data.findings[i];
    if (!isObject(finding)) {
      return { ok: false, error: `Finding at index ${i} is not an object.` };
    }
    if (!isSeverity(finding.severity)) {
      return {
        ok: false,
        error: `Finding at index ${i} has an invalid "severity" (expected one of: ${SEVERITIES.join(', ')}).`,
      };
    }
  }
  return { ok: true, report: data as unknown as ReportData };
}

function getSummaryCount(summary: Summary, severity: Severity): number {
  const value = summary[severity];
  return typeof value === 'number' ? value : 0;
}

function getTotal(summary: Summary, findings: Finding[]): number {
  if (typeof summary.total === 'number') {
    return summary.total;
  }
  return findings.length;
}

export default function Report() {
  const [report, setReport] = useState<ReportData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFile = useCallback((file: File) => {
    setError(null);
    setReport(null);
    setFileName(file.name);

    const reader = new FileReader();
    reader.onload = () => {
      const text = typeof reader.result === 'string' ? reader.result : '';
      let parsed: unknown;
      try {
        parsed = JSON.parse(text);
      } catch {
        setError('The file is not valid JSON. Make sure it was generated with "depradar --format json".');
        return;
      }
      const result = validateReport(parsed);
      if (!result.ok) {
        setError(`Invalid report file: ${result.error}`);
        return;
      }
      setReport(result.report);
    };
    reader.onerror = () => {
      setError('Failed to read the file.');
    };
    reader.readAsText(file);
  }, []);

  const onInputChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (file) {
        handleFile(file);
      }
      event.target.value = '';
    },
    [handleFile],
  );

  const onDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      setIsDragging(false);
      const file = event.dataTransfer.files?.[0];
      if (file) {
        handleFile(file);
      }
    },
    [handleFile],
  );

  const onDragOver = useCallback((event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setIsDragging(true);
  }, []);

  const onDragLeave = useCallback((event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setIsDragging(false);
  }, []);

  const grouped = useMemo(() => {
    const groups: Record<Severity, Finding[]> = {
      critical: [],
      high: [],
      medium: [],
      low: [],
      info: [],
    };
    if (report) {
      for (const finding of report.findings) {
        groups[finding.severity].push(finding);
      }
    }
    return groups;
  }, [report]);

  const reset = useCallback(() => {
    setReport(null);
    setError(null);
    setFileName(null);
  }, []);

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      <header className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Scan Report Viewer</h1>
        <p className="mt-1 text-sm text-gray-600">
          Drop or upload a <code className="rounded bg-gray-100 px-1">depradar --format json</code> file to view its
          summary and findings. Everything is processed locally in your browser.
        </p>
      </header>

      <div
        role="button"
        tabIndex={0}
        aria-label="Upload report file"
        onClick={() => inputRef.current?.click()}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            inputRef.current?.click();
          }
        }}
        onDrop={onDrop}
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        className={`flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed p-8 text-center transition-colors ${
          isDragging ? 'border-blue-500 bg-blue-50' : 'border-gray-300 bg-gray-50 hover:border-gray-400'
        }`}
      >
        <p className="text-sm font-medium text-gray-700">Drop your JSON report here, or click to select a file</p>
        <p className="mt-1 text-xs text-gray-500">{fileName ? `Selected: ${fileName}` : 'No file selected'}</p>
        <input
          ref={inputRef}
          type="file"
          accept="application/json,.json"
          className="hidden"
          data-testid="report-file-input"
          onChange={onInputChange}
        />
      </div>

      {error && (
        <div role="alert" className="mt-4 rounded-md border border-red-300 bg-red-50 p-4 text-sm text-red-800">
          {error}
        </div>
      )}

      {report && (
        <section className="mt-8">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-gray-900">Summary</h2>
            <button
              type="button"
              onClick={reset}
              className="rounded-md border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-100"
            >
              Clear
            </button>
          </div>

          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-6">
            <div className="rounded-md border border-gray-300 bg-white p-3 text-center">
              <div className="text-2xl font-bold text-gray-900">{getTotal(report.summary, report.findings)}</div>
              <div className="text-xs uppercase tracking-wide text-gray-500">Total</div>
            </div>
            {SEVERITIES.map((severity) => (
              <div
                key={severity}
                className={`rounded-md border p-3 text-center ${SEVERITY_STYLES[severity]}`}
              >
                <div className="text-2xl font-bold">{getSummaryCount(report.summary, severity)}</div>
                <div className="text-xs uppercase tracking-wide">{severity}</div>
              </div>
            ))}
          </div>

          <div className="mt-8 space-y-6">
            {SEVERITIES.map((severity) => {
              const findings = grouped[severity];
              if (findings.length === 0) {
                return null;
              }
              return (
                <div key={severity}>
                  <h3 className="mb-2 flex items-center gap-2 text-md font-semibold text-gray-900">
                    <span className={`rounded border px-2 py-0.5 text-xs uppercase ${SEVERITY_STYLES[severity]}`}>
                      {severity}
                    </span>
                    <span className="text-sm font-normal text-gray-500">({findings.length})</span>
                  </h3>
                  <ul className="space-y-2">
                    {findings.map((finding, index) => (
                      <li
                        key={finding.id ?? `${severity}-${index}`}
                        className="rounded-md border border-gray-200 bg-white p-4"
                      >
                        <div className="flex flex-wrap items-baseline justify-between gap-2">
                          <span className="font-medium text-gray-900">
                            {finding.title ?? finding.name ?? finding.package ?? 'Untitled finding'}
                          </span>
                          {(finding.package || finding.version) && (
                            <span className="text-sm text-gray-600">
                              {finding.package}
                              {finding.version ? `@${finding.version}` : ''}
                            </span>
                          )}
                        </div>
                        {finding.description && (
                          <p className="mt-1 text-sm text-gray-700">{finding.description}</p>
                        )}
                        <div className="mt-2 flex flex-wrap gap-3 text-xs text-gray-500">
                          {finding.cve && <span>CVE: {finding.cve}</span>}
                          {finding.url && (
                            <a
                              href={finding.url}
                              target="_blank"
                              rel="noreferrer noopener"
                              className="text-blue-600 hover:underline"
                            >
                              Details
                            </a>
                          )}
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              );
            })}
            {report.findings.length === 0 && (
              <p className="text-sm text-gray-600">No findings reported. 🎉</p>
            )}
          </div>
        </section>
      )}
    </div>
  );
}