'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { Dag } = require('../index.js');

// ── helpers ───────────────────────────────────────────────────────────────────

function makeChain() {
  const dag = new Dag();
  const n1 = dag.addNode('a');
  const n2 = dag.addNode('b');
  const n3 = dag.addNode('c');
  dag.addEdge(n1, n2, null);
  dag.addEdge(n2, n3, null);
  return { dag, n1, n2, n3 };
}

// ── basic construction ────────────────────────────────────────────────────────

test('addNode and nodeMeta', () => {
  const dag = new Dag();
  const n = dag.addNode({ key: 'value' });
  assert.deepEqual(dag.nodeMeta(n), { key: 'value' });
});

test('addEdge and edgeMeta', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, 42);
  assert.equal(dag.edgeMeta(e), 42);
});

test('setNodeMeta', () => {
  const dag = new Dag();
  const n = dag.addNode(1);
  dag.setNodeMeta(n, 2);
  assert.equal(dag.nodeMeta(n), 2);
});

test('setEdgeMeta', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, 'old');
  dag.setEdgeMeta(e, 'new');
  assert.equal(dag.edgeMeta(e), 'new');
});

// ── error cases ───────────────────────────────────────────────────────────────

test('nodeNotFound throws', () => {
  const dag = new Dag();
  const n = dag.addNode(null);
  dag.removeNode(n);
  assert.throws(() => dag.nodeMeta(n), /not found/i);
});

test('edgeNotFound throws', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.throws(() => dag.edgeMeta(e), /not found/i);
});

test('cycle detection throws', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  dag.addEdge(n1, n2, null);
  assert.throws(() => dag.addEdge(n2, n1, null), /cycle/i);
});

test('self-loop throws', () => {
  const dag = new Dag();
  const n = dag.addNode(null);
  assert.throws(() => dag.addEdge(n, n, null), /cycle/i);
});

// ── removeEdge ────────────────────────────────────────────────────────────────

test('removeEdge disconnects nodes', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.equal(dag.hasPath(n1, n2), false);
});

test('removeEdge preserves nodes', () => {
  const dag = new Dag();
  const n1 = dag.addNode('x');
  const n2 = dag.addNode('y');
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.equal(dag.nodeMeta(n1), 'x');
  assert.equal(dag.nodeMeta(n2), 'y');
});

test('removeEdge nonexistent throws', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.throws(() => dag.removeEdge(e), /not found/i);
});

test('removeEdge cleans up adjacency', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const edges = dag.edges();
  const e = edges.find(eid => {
    const ep = dag.edgeEndpoints(eid);
    return ep[0] === n1 && ep[1] === n2;
  });
  dag.removeEdge(e);
  assert.ok(dag.roots().includes(n1));
  assert.ok(dag.leaves().includes(n1));
  assert.ok(dag.roots().includes(n2));
});

// ── nodes / edges ─────────────────────────────────────────────────────────────

test('nodes empty', () => {
  assert.deepEqual(new Dag().nodes(), []);
});

test('nodes returns all', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const nodes = dag.nodes();
  assert.equal(nodes.length, 3);
  assert.ok(nodes.includes(n1));
  assert.ok(nodes.includes(n2));
  assert.ok(nodes.includes(n3));
});

test('nodes after removeNode', () => {
  const { dag, n1, n2, n3 } = makeChain();
  dag.removeNode(n2);
  const nodes = dag.nodes();
  assert.ok(nodes.includes(n1));
  assert.ok(!nodes.includes(n2));
  assert.ok(nodes.includes(n3));
});

test('edges empty', () => {
  assert.deepEqual(new Dag().edges(), []);
});

test('edges returns all', () => {
  const { dag } = makeChain();
  assert.equal(dag.edges().length, 2);
});

test('edges after removeEdge', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.deepEqual(dag.edges(), []);
});

// ── edgeEndpoints ─────────────────────────────────────────────────────────────

test('edgeEndpoints returns [from, to]', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  const [from, to] = dag.edgeEndpoints(e);
  assert.equal(from, n1);
  assert.equal(to, n2);
});

test('edgeEndpoints nonexistent throws', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  dag.removeEdge(e);
  assert.throws(() => dag.edgeEndpoints(e), /not found/i);
});

// ── ancestors / descendants ───────────────────────────────────────────────────

test('ancestors', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const anc = dag.ancestors(n3);
  assert.equal(anc.length, 2);
  assert.ok(anc.includes(n1));
  assert.ok(anc.includes(n2));
});

test('descendants', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const desc = dag.descendants(n1);
  assert.equal(desc.length, 2);
  assert.ok(desc.includes(n2));
  assert.ok(desc.includes(n3));
});

// ── roots / leaves ────────────────────────────────────────────────────────────

test('roots and leaves', () => {
  const { dag, n1, n3 } = makeChain();
  assert.ok(dag.roots().includes(n1));
  assert.ok(dag.leaves().includes(n3));
});

// ── topologicalSort ───────────────────────────────────────────────────────────

test('topologicalSort order', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const order = dag.topologicalSort();
  assert.ok(order.indexOf(n1) < order.indexOf(n2));
  assert.ok(order.indexOf(n2) < order.indexOf(n3));
});

test('topologicalSort empty', () => {
  assert.deepEqual(new Dag().topologicalSort(), []);
});

// ── hasPath ───────────────────────────────────────────────────────────────────

test('hasPath true and false', () => {
  const { dag, n1, n3 } = makeChain();
  assert.equal(dag.hasPath(n1, n3), true);
  assert.equal(dag.hasPath(n3, n1), false);
});

// ── serialisation ─────────────────────────────────────────────────────────────

test('toJson produces valid JSON', () => {
  const { dag } = makeChain();
  const parsed = JSON.parse(dag.toJson());
  assert.equal(typeof parsed, 'object');
});

test('fromJson roundtrip preserves structure', () => {
  const { dag, n1, n2, n3 } = makeChain();
  const dag2 = Dag.fromJson(dag.toJson());

  assert.equal(dag2.nodeMeta(n1), 'a');
  assert.equal(dag2.nodeMeta(n2), 'b');
  assert.equal(dag2.nodeMeta(n3), 'c');
  assert.equal(dag2.hasPath(n1, n3), true);
  assert.equal(dag2.hasPath(n3, n1), false);
});

test('fromJson empty roundtrip', () => {
  const dag2 = Dag.fromJson(new Dag().toJson());
  assert.deepEqual(dag2.nodes(), []);
  assert.deepEqual(dag2.edges(), []);
});

test('fromJson preserves edge endpoints', () => {
  const dag = new Dag();
  const n1 = dag.addNode(null);
  const n2 = dag.addNode(null);
  const e = dag.addEdge(n1, n2, null);
  const dag2 = Dag.fromJson(dag.toJson());
  const [from, to] = dag2.edgeEndpoints(e);
  assert.equal(from, n1);
  assert.equal(to, n2);
});

test('fromJson invalid throws', () => {
  assert.throws(() => Dag.fromJson('not valid json'));
});
