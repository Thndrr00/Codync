(function (expectedOrigin, kind, value) {
  const field = this;
  const doc = field.ownerDocument;
  const view = doc && doc.defaultView;
  // OpenBot 0.69: a form control named `hidden` or `inert` shadows the element's own property. Read
  // those attributes from the prototype so a visible field cannot be mistaken for a hidden one, and
  // never send a secret into an inert subtree.
  const hasAttribute = view && view.Element && view.Element.prototype.hasAttribute;
  function marked(node) {
    try { return hasAttribute.call(node, 'hidden') || hasAttribute.call(node, 'inert'); } catch (error) { return false; }
  }
  const inHiddenTree = field.closest && field.closest('[hidden],[inert]');
  const ready = !!view && view.top === view && doc.location.protocol === 'https:' &&
    doc.location.origin === expectedOrigin && field instanceof view.HTMLInputElement &&
    doc.activeElement === field && field.isConnected && !field.disabled && !field.readOnly &&
    !marked(field) && !inHiddenTree && field.getClientRects().length > 0 &&
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
