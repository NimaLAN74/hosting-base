import React, { useState, useRef, useEffect } from 'react';
import './MultiSelect.css';

/**
 * Multi-select dropdown component
 */
const MultiSelect = ({ 
  options = [], 
  selected = [], 
  onChange, 
  placeholder = 'Select...',
  label = '',
  searchable = true 
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const dropdownRef = useRef(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target)) {
        setIsOpen(false);
        setSearchTerm('');
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const filteredOptions = searchable && searchTerm
    ? options.filter(opt => 
        String(opt.label || opt.value || opt).toLowerCase().includes(searchTerm.toLowerCase())
      )
    : options;

  const handleToggle = (value) => {
    const newSelected = selected.includes(value)
      ? selected.filter(v => v !== value)
      : [...selected, value];
    onChange(newSelected);
  };

  const handleSelectAll = () => {
    if (selected.length === filteredOptions.length) {
      onChange([]);
    } else {
      onChange(filteredOptions.map(opt => opt.value || opt));
    }
  };

  const handleClear = () => {
    onChange([]);
  };

  const getDisplayText = () => {
    if (selected.length === 0) return placeholder;
    if (selected.length === 1) {
      const option = options.find(opt => (opt.value || opt) === selected[0]);
      return option?.label || option?.value || selected[0];
    }
    return `${selected.length} selected`;
  };

  return (
    <div className="multi-select" ref={dropdownRef}>
      {label && <label className="multi-select-label">{label}</label>}
      <div 
        className={`multi-select-dropdown ${isOpen ? 'open' : ''}`}
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className="multi-select-value">{getDisplayText()}</span>
        <span className="multi-select-arrow">▼</span>
      </div>
      
      {isOpen && (
        <div className="multi-select-menu">
          {searchable && (
            <div className="multi-select-search">
              <input
                type="text"
                placeholder="Search..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                onClick={(e) => e.stopPropagation()}
                autoFocus
              />
            </div>
          )}
          
          <div className="multi-select-actions">
            <button 
              type="button"
              className="multi-select-action-btn"
              onClick={(e) => {
                e.stopPropagation();
                handleSelectAll();
              }}
            >
              {selected.length === filteredOptions.length ? 'Deselect All' : 'Select All'}
            </button>
            {selected.length > 0 && (
              <button 
                type="button"
                className="multi-select-action-btn"
                onClick={(e) => {
                  e.stopPropagation();
                  handleClear();
                }}
              >
                Clear
              </button>
            )}
          </div>

          <div className="multi-select-options">
            {filteredOptions.length === 0 ? (
              <div className="multi-select-no-results">No options found</div>
            ) : (
              filteredOptions.map((option) => {
                const value = option.value || option;
                const label = option.label || option.value || option;
                const isSelected = selected.includes(value);
                
                return (
                  <div
                    key={value}
                    className={`multi-select-option ${isSelected ? 'selected' : ''}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      // Only toggle if click is not directly on checkbox (checkbox handles its own click)
                      if (e.target.type !== 'checkbox') {
                        handleToggle(value);
                      }
                    }}
                  >
                    <label style={{ display: 'flex', alignItems: 'center', cursor: 'pointer', width: '100%' }}>
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onChange={(e) => {
                          e.stopPropagation();
                          handleToggle(value);
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                        }}
                        style={{ marginRight: '10px', cursor: 'pointer' }}
                      />
                      <span style={{ flex: 1 }}>{label}</span>
                    </label>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default MultiSelect;
