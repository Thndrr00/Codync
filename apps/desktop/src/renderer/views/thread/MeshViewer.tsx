import { useEffect, useRef, useState } from 'react'
import * as THREE from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'

/**
 * A GLB played in the chat: its geometry is read from the file the card already has (a blob URL),
 * framed, lit and turned slowly. Other mesh formats keep their download card.
 */
export function MeshViewer({ url }: { url: string | null }) {
  const tr = useTr()
  const canvas = useRef<HTMLCanvasElement>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    const element = canvas.current
    if (!url || !element) return
    const scene = new THREE.Scene()
    scene.background = new THREE.Color(0x111111)
    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000)
    const renderer = new THREE.WebGLRenderer({ canvas: element, antialias: true })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    scene.add(new THREE.HemisphereLight(0xffffff, 0x333333, 2.5))
    const key = new THREE.DirectionalLight(0xffffff, 2)
    key.position.set(3, 5, 4)
    scene.add(key)
    const fill = new THREE.DirectionalLight(0xffffff, 0.8)
    fill.position.set(-4, -2, -3)
    scene.add(fill)

    let frame = 0
    let disposed = false
    let model: THREE.Object3D | null = null
    const resize = () => {
      const width = element.clientWidth || 640
      const height = element.clientHeight || 360
      renderer.setSize(width, height, false)
      camera.aspect = width / height
      camera.updateProjectionMatrix()
    }
    const observer = new ResizeObserver(resize)
    observer.observe(element)
    resize()

    new GLTFLoader().load(
      url,
      (gltf) => {
        if (disposed) return
        model = gltf.scene
        scene.add(model)
        const box = new THREE.Box3().setFromObject(model)
        const size = box.getSize(new THREE.Vector3()).length() || 1
        const center = box.getCenter(new THREE.Vector3())
        model.position.sub(center)
        camera.position.set(size * 0.9, size * 0.45, size * 1.3)
        camera.lookAt(0, 0, 0)
      },
      undefined,
      () => setFailed(true),
    )

    const tick = () => {
      frame = requestAnimationFrame(tick)
      if (model) model.rotation.y += 0.003
      renderer.render(scene, camera)
    }
    tick()

    return () => {
      disposed = true
      cancelAnimationFrame(frame)
      observer.disconnect()
      scene.traverse((object) => {
        if (!(object instanceof THREE.Mesh)) return
        object.geometry.dispose()
        const material = object.material
        Array.isArray(material) ? material.forEach((item) => item.dispose()) : material.dispose()
      })
      renderer.dispose()
    }
  }, [url])

  if (failed) {
    return <Text size="caption" tone="attention">{tr('This 3D file could not be read here. Download it to open it in a 3D viewer.')}</Text>
  }
  return (
    <div className="attachment-mesh">
      <canvas ref={canvas} aria-label={tr('3D model')} />
    </div>
  )
}
