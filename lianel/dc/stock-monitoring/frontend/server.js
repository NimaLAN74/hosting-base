// Minimal static server for stock-monitoring UI under /stock (no API proxy).
const express = require('express');
const path = require('path');

const app = express();
const buildPath = path.join(__dirname, 'build');
app.use('/stock', express.static(buildPath, { index: false }));
app.get('/stock', (req, res) => res.sendFile(path.join(buildPath, 'index.html')));
app.get('/stock/*', (req, res) => res.sendFile(path.join(buildPath, 'index.html')));
app.get('/', (req, res) => res.redirect(302, '/stock'));
app.get('*', (req, res) => res.sendFile(path.join(buildPath, 'index.html')));

const PORT = process.env.PORT || 80;
app.listen(PORT, '0.0.0.0', () => {
  console.log(`Stock Monitoring UI listening on port ${PORT}`);
});
