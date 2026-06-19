// Fake print-shop data for the Frappe UI kit. Plain globals (no modules).
window.FrappeData = (function () {
  const orders = [
    { id: 1048, customer: 'Acme Co.', contact: 'Dana Ruiz', job: '500 matte business cards', qty: 500, stock: '16pt Matte', status: 'press', total: 184.00, due: 'Today, 4:00pm', rush: true, stage: 'On press', assignees: ['Max Bowen'] },
    { id: 1047, customer: 'Northwind Cafe', contact: 'Priya Shah', job: 'A-frame sidewalk banner', qty: 2, stock: '13oz Vinyl', status: 'art', total: 240.00, due: 'Tomorrow', rush: false, stage: 'Awaiting art', assignees: ['Dana Ruiz'] },
    { id: 1046, customer: 'Lumen Studio', contact: 'Lee Ortiz', job: 'Tri-fold brochures', qty: 1000, stock: '100lb Gloss', status: 'queued', total: 612.50, due: 'Fri', rush: false, stage: 'Queued', assignees: [] },
    { id: 1045, customer: 'Harbor Yoga', contact: 'Sam Kade', job: 'Vinyl window decals', qty: 12, stock: 'Cut Vinyl', status: 'queued', total: 96.00, due: 'Fri', rush: false, stage: 'Prepress', assignees: ['Priya Shah'] },
    { id: 1044, customer: 'Acme Co.', contact: 'Dana Ruiz', job: 'Letterhead reprint', qty: 250, stock: '70lb Uncoated', status: 'shipped', total: 78.00, due: 'Shipped', rush: false, stage: 'Shipped', assignees: ['Max Bowen'] },
    { id: 1043, customer: 'Pine & Oak', contact: 'Jo Tran', job: 'Foil wedding invites', qty: 120, stock: '120lb Cotton', status: 'press', total: 540.00, due: 'Mon', rush: false, stage: 'Bindery', assignees: ['Lee Ortiz', 'Sam Kade'] },
    { id: 1042, customer: 'Bright Labs', contact: 'Avery Cole', job: 'Roll-up retractable banner', qty: 1, stock: 'Polyester', status: 'overdue', total: 159.00, due: 'Overdue 1d', rush: true, stage: 'On press', assignees: ['Max Bowen'] },
    { id: 1041, customer: 'Cedar Dental', contact: 'Rae Kim', job: 'Appointment cards', qty: 2000, stock: '14pt Gloss', status: 'queued', total: 220.00, due: 'Next Tue', rush: false, stage: 'Queued', assignees: [] },
  ];

  const invoices = [
    { id: 'INV-2048', customer: 'Acme Co.', order: 1048, amount: 184.00, status: 'sent', date: 'Jun 18', deposit: 50 },
    { id: 'INV-2047', customer: 'Northwind Cafe', order: 1047, amount: 240.00, status: 'paid', date: 'Jun 17', deposit: 240 },
    { id: 'INV-2046', customer: 'Bright Labs', order: 1042, amount: 159.00, status: 'overdue', date: 'Jun 10', deposit: 0 },
    { id: 'INV-2045', customer: 'Pine & Oak', order: 1043, amount: 540.00, status: 'deposit', date: 'Jun 15', deposit: 270 },
    { id: 'INV-2044', customer: 'Lumen Studio', order: 1046, amount: 612.50, status: 'draft', date: '—', deposit: 0 },
    { id: 'INV-2043', customer: 'Acme Co.', order: 1044, amount: 78.00, status: 'paid', date: 'Jun 12', deposit: 78 },
  ];

  // Kanban columns for production
  const columns = [
    { key: 'Queued', tone: 'info' },
    { key: 'Prepress', tone: 'brand' },
    { key: 'On press', tone: 'brand' },
    { key: 'Bindery', tone: 'warning' },
    { key: 'Shipped', tone: 'success' },
  ];

  const statusBadge = {
    queued:  { tone: 'info', label: 'Queued' },
    art:     { tone: 'warning', label: 'Awaiting art' },
    press:   { tone: 'brand', label: 'On press' },
    shipped: { tone: 'success', label: 'Shipped' },
    overdue: { tone: 'danger', label: 'Overdue' },
  };

  const invoiceBadge = {
    draft:   { tone: 'neutral', label: 'Draft' },
    sent:    { tone: 'info', label: 'Sent' },
    paid:    { tone: 'success', label: 'Paid' },
    overdue: { tone: 'danger', label: 'Overdue' },
    deposit: { tone: 'warning', label: 'Deposit paid' },
  };

  const money = (n) => '$' + n.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

  return { orders, invoices, columns, statusBadge, invoiceBadge, money };
})();
