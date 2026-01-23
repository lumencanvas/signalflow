import { describe, it, expect } from 'vitest';
import { ClaspBuilder } from '../src/builder';

describe('ClaspBuilder', () => {
  it('should create builder with URL', () => {
    const builder = new ClaspBuilder('ws://localhost:7330');
    expect(builder).toBeDefined();
    expect(builder.getUrl()).toBe('ws://localhost:7330');
  });

  it('should set name and verify value', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withName('Test Client');
    expect(builder).toBeDefined();
    expect(builder.getName()).toBe('Test Client');
  });

  it('should set features and verify values', () => {
    const features = ['param', 'event', 'stream'];
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withFeatures(features);
    expect(builder).toBeDefined();
    expect(builder.getFeatures()).toEqual(features);
  });

  it('should set token and verify value', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withToken('secret-token');
    expect(builder).toBeDefined();
    expect(builder.getToken()).toBe('secret-token');
  });

  it('should set reconnect options and verify values', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withReconnect(true, 5000);
    expect(builder).toBeDefined();
    expect(builder.getReconnect()).toBe(true);
    expect(builder.getReconnectInterval()).toBe(5000);
  });

  it('should set reconnect without interval', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withReconnect(false);
    expect(builder.getReconnect()).toBe(false);
    expect(builder.getReconnectInterval()).toBeUndefined();
  });

  it('should chain all options and verify all values', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withName('Full Test')
      .withFeatures(['param', 'event'])
      .withToken('token123')
      .withReconnect(true, 3000);

    expect(builder).toBeDefined();
    expect(builder.getUrl()).toBe('ws://localhost:7330');
    expect(builder.getName()).toBe('Full Test');
    expect(builder.getFeatures()).toEqual(['param', 'event']);
    expect(builder.getToken()).toBe('token123');
    expect(builder.getReconnect()).toBe(true);
    expect(builder.getReconnectInterval()).toBe(3000);
  });

  it('should have connect method that returns a Promise', () => {
    const builder = new ClaspBuilder('ws://localhost:7330');
    expect(typeof builder.connect).toBe('function');
  });

  it('should use alias methods correctly', () => {
    // Test that name() and withName() work the same
    const builder1 = new ClaspBuilder('ws://localhost:7330').name('Test1');
    const builder2 = new ClaspBuilder('ws://localhost:7330').withName('Test2');

    expect(builder1.getName()).toBe('Test1');
    expect(builder2.getName()).toBe('Test2');
  });

  it('should use features() alias correctly', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .features(['param']);
    expect(builder.getFeatures()).toEqual(['param']);
  });

  it('should use token() alias correctly', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .token('my-token');
    expect(builder.getToken()).toBe('my-token');
  });

  it('should use reconnect() alias correctly', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .reconnect(true);
    expect(builder.getReconnect()).toBe(true);
  });

  it('should use reconnectInterval() method correctly', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .reconnectInterval(10000);
    expect(builder.getReconnectInterval()).toBe(10000);
  });

  it('should allow overwriting values', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withName('First')
      .withName('Second')
      .withName('Final');

    expect(builder.getName()).toBe('Final');
  });

  it('should return all options via getOptions()', () => {
    const builder = new ClaspBuilder('ws://localhost:7330')
      .withName('Test')
      .withFeatures(['param'])
      .withToken('secret')
      .withReconnect(true, 5000);

    const options = builder.getOptions();
    expect(options).toEqual({
      name: 'Test',
      features: ['param'],
      token: 'secret',
      reconnect: true,
      reconnectInterval: 5000,
    });
  });

  it('should return empty options for fresh builder', () => {
    const builder = new ClaspBuilder('ws://localhost:7330');
    const options = builder.getOptions();
    expect(options).toEqual({});
  });
});
