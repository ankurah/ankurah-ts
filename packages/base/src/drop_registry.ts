// TS-ONLY: Symbol.dispose polyfill and FinalizationRegistry for leak detection

export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;

export interface LeakInfo {
  label: string;
  creationStack: string;
  severity: 'fatal' | 'warning';
}

export const leakRegistry = new FinalizationRegistry<LeakInfo>((info) => {
  const message =
    `BUG: ${info.label} was garbage collected without being dropped.\n` +
    `This indicates a missing drop() call or a missing 'using' declaration.\n` +
    `Allocated at:\n${info.creationStack}`;

  if (info.severity === 'fatal') {
    queueMicrotask(() => {
      throw new Error(message);
    });
  } else {
    console.error(message);
  }
});
