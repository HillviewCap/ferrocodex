import { describe, expect, it } from 'vitest';
import { formatFirmwareFileSize, formatFirmwareHash } from '../firmware';

describe('formatFirmwareFileSize', () => {
  it('should format valid byte sizes correctly', () => {
    expect(formatFirmwareFileSize(0)).toBe('0 Bytes');
    expect(formatFirmwareFileSize(512)).toBe('512 Bytes');
    expect(formatFirmwareFileSize(1024)).toBe('1 KB');
    expect(formatFirmwareFileSize(1536)).toBe('1.5 KB');
    expect(formatFirmwareFileSize(1048576)).toBe('1 MB');
    expect(formatFirmwareFileSize(1073741824)).toBe('1 GB');
  });

  it('should handle null and undefined values', () => {
    expect(formatFirmwareFileSize(null)).toBe('0 Bytes');
    expect(formatFirmwareFileSize(undefined)).toBe('0 Bytes');
  });

  it('should handle NaN and invalid values', () => {
    expect(formatFirmwareFileSize(NaN)).toBe('0 Bytes');
    expect(formatFirmwareFileSize(-100)).toBe('0 Bytes');
    expect(formatFirmwareFileSize(Infinity)).toBe('0 Bytes');
  });

  it('should handle string numbers', () => {
    expect(formatFirmwareFileSize('1024' as any)).toBe('1 KB');
    expect(formatFirmwareFileSize('invalid' as any)).toBe('0 Bytes');
  });
});

describe('formatFirmwareHash', () => {
  it('should format valid hashes correctly', () => {
    const longHash = 'abcdef1234567890abcdef1234567890';
    expect(formatFirmwareHash(longHash)).toBe('abcdef12...');
    
    const shortHash = 'abc123';
    expect(formatFirmwareHash(shortHash)).toBe('abc123');
  });

  it('should handle null and undefined values', () => {
    expect(formatFirmwareHash(null)).toBe('N/A');
    expect(formatFirmwareHash(undefined)).toBe('N/A');
  });

  it('should handle empty and placeholder values', () => {
    expect(formatFirmwareHash('')).toBe('N/A');
    expect(formatFirmwareHash('   ')).toBe('N/A');
    expect(formatFirmwareHash('unknown')).toBe('N/A');
  });

  it('should handle edge case hashes', () => {
    expect(formatFirmwareHash('a')).toBe('a');
    expect(formatFirmwareHash('12345678')).toBe('12345678');
    expect(formatFirmwareHash('123456789')).toBe('12345678...');
  });
});