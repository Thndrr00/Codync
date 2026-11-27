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
  // OpenBot 0.70: some sign-in pages put a masked decoy in front of the real password input. This
  // pure decision reads only plain descriptors (never values) and mirrors `password_field` in
  // protected_browser.rs. An explicit current-password field wins, then a password input, then a
  // text input masked with -webkit-text-security. Hidden, disabled, read-only and invisible fields
  // are never chosen; the first best field wins a tie.
  // begin password_field
  function password_field(fields) {
    let best = -1;
    let score = -1;
    for (let i = 0; i < fields.length; i++) {
      const f = fields[i];
      if (!f || !f.visible || f.disabled || f.readOnly || f.hidden || f.inert) continue;
      if (f.type !== 'password' && !f.masked) continue;
      const rank = (f.autocomplete === 'current-password' ? 4 : 0) +
        (f.type === 'password' ? 2 : 0) + (f.masked ? 1 : 0);
      if (rank > score) { score = rank; best = i; }
    }
    return best;
  }
  // end password_field
  // OpenBot 0.78: an authenticator code field may name itself in its label or placeholder when
  // the page does not set `autocomplete=one-time-code`. Descriptors carry no field values.
  // begin code_field
  function code_field(fields) {
    let best = -1;
    let score = -1;
    for (let i = 0; i < fields.length; i++) {
      const f = fields[i];
      if (!f || !f.visible || f.disabled || f.readOnly || f.hidden || f.inert) continue;
      if (!['text', 'password', 'tel', 'number'].includes(f.type)) continue;
      const words = `${f.autocomplete || ''} ${f.label || ''} ${f.placeholder || ''}`.toLowerCase();
      const rank = (f.autocomplete === 'one-time-code' ? 4 : 0) +
        (/(authenticator|verification|one[- ]time|2fa|otp)/.test(words) ? 2 : 0) +
        (/(pin|code|passcode)/.test(words) ? 1 : 0);
      if (rank > score) { score = rank; best = i; }
    }
    return score >= 1 ? best : -1;
  }
  // end code_field

  // A password may be filled only into an input of the focused field's own form; a decoy outside
  // that form can never point the secret at an unrelated credential field.
  const form = kind === 'password' && field.closest ? field.closest('form') : null;
  const candidates = form && view ? Array.from(form.querySelectorAll('input')).slice(0, 40) : [field];
  const describe = (node) => {
    let style = null;
    try { style = view && view.getComputedStyle(node); } catch (error) { style = null; }
    const security = style && (style.webkitTextSecurity || style.textSecurity || '');
    return {
      type: node.type,
      autocomplete: node.autocomplete || '',
      masked: security === 'disc' || security === 'circle',
      visible: node.getClientRects().length > 0,
      disabled: !!node.disabled,
      readOnly: !!node.readOnly,
      hidden: marked(node),
      inert: !!(node.closest && node.closest('[hidden],[inert]'))
    };
  };
  const descriptors = candidates.map(describe);
  const chosen = kind === 'password' ? password_field(descriptors)
    : kind === 'code' ? code_field(descriptors) : 0;
  const target = chosen >= 0 ? candidates[chosen] : null;
  // The host probes this exact remote node BEFORE sending a secret to its execution context.
  const ready = !!view && view.top === view && doc.location.protocol === 'https:' &&
    doc.location.origin === expectedOrigin && !!target && target instanceof view.HTMLInputElement &&
    (kind === 'password'
      ? !!form && target.closest('form') === form
      : doc.activeElement === field && field === target && field.isConnected && !field.disabled &&
        !field.readOnly && !marked(field) && !inHiddenTree && field.getClientRects().length > 0 &&
        (kind !== 'code' || chosen >= 0) && ['text', 'password', 'tel', 'number'].includes(field.type)) &&
    !marked(target) && !(target.closest && target.closest('[hidden],[inert]')) &&
    !target.disabled && !target.readOnly && target.isConnected && target.getClientRects().length > 0;
  if (arguments.length < 3) return { ready, origin: doc ? doc.location.origin : null, fields: descriptors, chosen };
  if (!ready || typeof value !== 'string') return 'refused';
  if (target.type === 'number' && !/^\d+$/.test(value)) return 'refused';
  const setter = Object.getOwnPropertyDescriptor(view.HTMLInputElement.prototype, 'value').set;
  setter.call(target, value);
  if (target.value !== value) return 'refused';
  target.dispatchEvent(new view.Event('input', { bubbles: true }));
  target.dispatchEvent(new view.Event('change', { bubbles: true }));
  return 'filled';
})
