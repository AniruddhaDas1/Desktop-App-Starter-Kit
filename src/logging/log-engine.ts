import type { LogEntry, LogLevel, ModuleLogger } from './log-types';
import { LOG_LEVEL_PRIORITY } from './log-types';

// Renderer-side logger. Console output happens here; persistence is handled by
// tauri-plugin-log on the Rust side via the `log:send` command, so there is no
// file writer in the webview.
class LogEngine {
  private minLevel: LogLevel = 'debug';

  setMinLevel(level: LogLevel): void {
    this.minLevel = level;
  }

  debug(module: string, message: string, context?: Record<string, unknown>): void {
    this.write('debug', module, message, undefined, context);
  }

  info(module: string, message: string, context?: Record<string, unknown>): void {
    this.write('info', module, message, undefined, context);
  }

  warn(module: string, message: string, context?: Record<string, unknown>): void {
    this.write('warn', module, message, undefined, context);
  }

  error(module: string, message: string, error?: unknown, context?: Record<string, unknown>): void {
    this.write('error', module, message, error, context);
  }

  fatal(module: string, message: string, error?: unknown, context?: Record<string, unknown>): void {
    this.write('fatal', module, message, error, context);
  }

  /** Ingest a pre-formed entry (e.g. one forwarded from another module). */
  ingest(entry: LogEntry): void {
    if (LOG_LEVEL_PRIORITY[entry.level] < LOG_LEVEL_PRIORITY[this.minLevel]) return;
    void window.api
      ?.logSend({
        level: entry.level,
        module: entry.module,
        message: entry.message,
        ...(entry.context ? { context: entry.context } : {}),
        ...(entry.error ? { error: entry.error } : {}),
      })
      .catch(() => {});
  }

  createLogger(module: string): ModuleLogger {
    return {
      debug: (message, context) => this.debug(module, message, context),
      info: (message, context) => this.info(module, message, context),
      warn: (message, context) => this.warn(module, message, context),
      error: (message, error, context) => this.error(module, message, error, context),
      fatal: (message, error, context) => this.fatal(module, message, error, context),
    };
  }

  private write(
    level: LogLevel,
    module: string,
    message: string,
    error?: unknown,
    context?: Record<string, unknown>,
  ): void {
    if (LOG_LEVEL_PRIORITY[level] < LOG_LEVEL_PRIORITY[this.minLevel]) return;

    const prefix = `[${level.toUpperCase()}] [${module}]`;
    if (level === 'error' || level === 'fatal') console.error(prefix, message, error ?? '');
    else if (level === 'warn') console.warn(prefix, message);
    else console.log(prefix, message);

    this.ingest({
      level,
      module,
      message,
      timestamp: new Date().toISOString(),
      process: 'renderer',
      ...(context ? { context } : {}),
      ...(error
        ? {
            error:
              error instanceof Error
                ? { name: error.name, message: error.message, stack: error.stack }
                : { name: 'Error', message: String(error) },
          }
        : {}),
    });
  }
}

export const logEngine = new LogEngine();
