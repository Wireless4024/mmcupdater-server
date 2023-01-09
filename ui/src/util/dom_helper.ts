/**
 * find if [node] is a child of [element]
 * @param element
 * @param node
 */
export function was_child_of(element: Node, node: Node | null): boolean {
	return find_parent(node, it => it == element) != null
}

export function find_parent<T extends Node>(element: Node | null,
                                               cond: (elem: Node) => boolean): T | null {
	let ptr: Node | null = element
	while (ptr) {
		if (cond(ptr)) return ptr as T
		ptr = ptr.parentElement
	}
	return null
}