const EU_LOCALE = 'en-GB';

/** Use these for all date inputs so UI never shows US format (mm/dd/yyyy). */
export const EU_DATE_INPUT_PLACEHOLDER = 'DD/MM/YYYY (e.g. 31/12/2025)';
export const EU_DATE_INPUT_LABEL = 'DD/MM/YYYY';

const sanitizeDateInput = (value) => {
  if (!value) return '';
  return value.trim();
};

const toDate = (value) => {
  if (!value) return null;
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value;
  const str = String(value).trim();
  // Parse ISO date-only (YYYY-MM-DD) as local date to avoid timezone shifting the day
  const dateOnlyMatch = /^(\d{4})-(\d{2})-(\d{2})$/.exec(str);
  if (dateOnlyMatch) {
    const [, y, m, d] = dateOnlyMatch;
    const date = new Date(parseInt(y, 10), parseInt(m, 10) - 1, parseInt(d, 10));
    return Number.isNaN(date.getTime()) ? null : date;
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
};

export const formatDateEU = (value) => {
  const date = toDate(value);
  if (!date) return 'N/A';
  return date.toLocaleDateString(EU_LOCALE, { day: '2-digit', month: '2-digit', year: 'numeric' });
};

export const formatDateTimeEU = (value, { includeSeconds = true } = {}) => {
  const date = toDate(value);
  if (!date) return 'N/A';
  const options = {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  };
  if (includeSeconds) options.second = '2-digit';
  return date.toLocaleString(EU_LOCALE, options);
};

export const formatFilenameDateEU = (value = new Date()) =>
  formatDateEU(value).replace(/\//g, '-');

export const toISODateFromInput = (value) => {
  const trimmed = sanitizeDateInput(value);
  if (!trimmed) return null;
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
    return trimmed;
  }
  if (/^\d{2}[/-]\d{2}[/-]\d{4}$/.test(trimmed)) {
    const [day, month, year] = trimmed.split(/[/-]/);
    return `${year}-${month}-${day}`;
  }
  if (/^\d{8}$/.test(trimmed)) {
    const day = trimmed.substring(0, 2);
    const month = trimmed.substring(2, 4);
    const year = trimmed.substring(4, 8);
    return `${year}-${month}-${day}`;
  }
  return null;
};

export const isoToEUDateString = (iso) => {
  if (!iso) return '';
  return formatDateEU(`${iso}T00:00:00Z`);
};

export const isoInputFromEUString = (value) => {
  const iso = toISODateFromInput(value);
  return iso || '';
};

export const euStringFromISOInput = (iso) => {
  if (!iso) return '';
  const [year, month, day] = iso.split('-');
  if (!year || !month || !day) return '';
  return `${day}/${month}/${year}`;
};
