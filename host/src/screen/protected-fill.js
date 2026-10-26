(function (expectedOrigin, kind, value) {
  const field = this;
  const doc = field.ownerDocument;
  const view = doc && doc.defaultView;
  const ready = !!view && view.top === view && doc.location.protocol === 'https:' &&
    doc.location.origin === expectedOrigin && field instanceof view.HTMLInputElement &&
    doc.activeElement === field && field.isConnected && !field.disabled && !field.readOnly &&
    field.getClientRects().length > 0 &&
    (kind === 'password' ? field.type === 'password' : kind === 'code' &&
      field.autocomplete === 'one-time-code' && ['text', 'password', 'tel', 'number'].includes(field.type));
  // The host probes this exact remote node BEFORE sending a secret to its execution context.
  if (arguments.length < 3) return { ready, origin: doc ? doc.location.origin : null };
  if (!ready || typeof value !== 'string') return 'refused';
  if (field.type === 'number' && !/^\d+$/.test(value)) return 'refused';
  const setter = Object.getOwnPropertyDescriptor(view.HTMLInputElement.prototype, 'value').set;
  setter.call(field, value);
  if (field.value !== value) return 'refused';
  field.dispatchEvent(new view.Event('input', { bubbles: true }));
  field.dispatchEvent(new view.Event('change', { bubbles: true }));
  return 'filled';
})
