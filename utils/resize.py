from PIL import Image
import os
import json

# --- 配置 ---

input_file = 'utils/src.png'
output_folder = 'utils/output/'

# (1) 您的原始 PNG 目标
target_pngs = [('128x128.png', 128),
               ('128x128@2x.png', 256),
               ('32x32.png', 32),
               ('icon.png', 512),
               ('Square107x107Logo.png', 107),
               ('Square142x142Logo.png', 142),
               ('Square150x150Logo.png', 150),
               ('Square284x284Logo.png', 284),
               ('Square30x30Logo.png', 30),
               ('Square310x310Logo.png', 310),
               ('Square44x44Logo.png', 44),
               ('Square71x71Logo.png', 71),
               ('Square89x89Logo.png', 89),
               ('StoreLogo.png', 50),
               ('mipmap-hdpi/ic_launcher.png', 72),
               ('mipmap-mdpi/ic_launcher.png', 48),
               ('mipmap-xhdpi/ic_launcher.png', 96),
               ('mipmap-xxhdpi/ic_launcher.png', 144),
               ('mipmap-xxxhdpi/ic_launcher.png', 192),
               ('mipmap-hdpi/ic_launcher_round.png', 72),
               ('mipmap-hdpi/ic_launcher_foreground.png', 162),
               ('mipmap-mdpi/ic_launcher_round.png', 48),
               ('mipmap-mdpi/ic_launcher_foreground.png', 108),
               ('mipmap-xhdpi/ic_launcher_round.png', 96),
               ('mipmap-xhdpi/ic_launcher_foreground.png', 216),
               ('mipmap-xxhdpi/ic_launcher_round.png', 144),
               ('mipmap-xxhdpi/ic_launcher_foreground.png', 324),
               ('mipmap-xxxhdpi/ic_launcher_round.png', 192),
               ('mipmap-xxxhdpi/ic_launcher_foreground.png', 432),
               ]

# (2) ICO 图标需要的尺寸
# 需求：16, 24, 32, 48, 64, 256。32px 为第一层。
ico_sizes = [(32, 32), (16, 16), (24, 24), (48, 48), (64, 64), (256, 256)]

# (3) ICNS (macOS) 配置
icns_config = {
    "16x16": {"name": "icon_16x16.png", "size": 16, "ostype": "is32"},
    "16x16@2x": {"name": "icon_16x16@2x.png", "size": 32, "ostype": "ic11"},
    "32x32": {"name": "icon_32x32.png", "size": 32, "ostype": "il32"},
    "32x32@2x": {"name": "icon_32x32@2x.png", "size": 64, "ostype": "ic12"},
    "128x128": {"name": "icon_128x128.png", "size": 128, "ostype": "ic07"},
    "128x128@2x": {"name": "icon_128x128@2x.png", "size": 256, "ostype": "ic13"},
    "256x256": {"name": "icon_256x256.png", "size": 256, "ostype": "ic08"},
    "256x256@2x": {"name": "icon_256x256@2x.png", "size": 512, "ostype": "ic14"},
    "512x512": {"name": "icon_512x512.png", "size": 512, "ostype": "ic09"},
    "512x512@2x": {"name": "icon_512x512@2x.png", "size": 1024, "ostype": "ic10"}
}

# --- 检查 Pillow 版本以确定重采样方法 ---
try:
    # 现代 Pillow (>=10.0)
    from PIL.Image import Resampling
    NEAREST = Resampling.NEAREST
    LANCZOS = Resampling.LANCZOS
except ImportError:
    # 旧版 Pillow (<10.0)
    NEAREST = Image.NEAREST  # 值为 0
    LANCZOS = Image.LANCZOS  # 值为 1

resample_key = 'resample'
# --- 准备工作 ---

# 加载原始图像
try:
    image = Image.open(input_file)
except FileNotFoundError:
    print(f"错误: 输入文件 '{input_file}' 未找到。")
    exit()
except Exception as e:
    print(f"打开图像时出错: {e}")
    exit()

print(f"原始图像模式: {image.mode}。 正在转换为 RGBA...")
image = image.convert('RGBA')
print("转换完成。")

# 确保输出目录存在
os.makedirs(output_folder, exist_ok=True)


# --- 1. 生成原始 PNG 列表 ---

print("正在生成 PNG 图标...")
for (fileName, size) in target_pngs:
    full_path = os.path.join(output_folder, fileName)

    # 确保子目录 (如 'mipmap-hdpi') 存在
    file_dir = os.path.dirname(full_path)
    if file_dir:
        os.makedirs(file_dir, exist_ok=True)

    # 按照您的原始代码，使用 NEAREST (resample=0) 算法
    resize_args = {resample_key: NEAREST}
    resized_image = image.resize((size, size), **resize_args)

    resized_image.save(full_path, format='PNG')

print(f"PNG 图标已生成到 '{output_folder}' 目录。")


# --- 2. 生成 .ico 文件 ---

print("\n正在生成 icon.ico...")
ico_path = os.path.join(output_folder, 'icon.ico')
try:
    # Pillow 的 save 方法会自动从原始图像 'image' 调整大小并创建所有图层
    # 它默认使用高质量的缩放算法
    image.save(ico_path, format='ICO', sizes=ico_sizes)
    print(f"icon.ico 已生成 (包含 {len(ico_sizes)} 个图层)。")
except Exception as e:
    print(f"生成 icon.ico 失败: {e}")

# --- 3. 生成 .icns 文件 ---

print(f"\n正在生成 icon.icns (使用 Pillow)...")
output_icns_path = os.path.join(output_folder, 'icon.icns')

try:
    # 1. 从配置中获取所有需要的 ICNS 尺寸 (去重并排序)
    icns_unique_sizes = sorted(
        list(set(item['size'] for item in icns_config.values())))
    # -> [16, 32, 64, 128, 256, 512, 1024]

    icns_images = []
    resize_args = {resample_key: LANCZOS}  # ICNS 使用高质量缩放

    # 2. 生成所有尺寸的 Image 对象
    for size in icns_unique_sizes:
        # print(f"  > 正在添加 {size}x{size} 图层...") # (如果需要详细日志，取消注释)
        resized_img = image.resize((size, size), **resize_args)
        icns_images.append(resized_img)

    # 3. 保存
    # 我们使用列表中的最后一个 (最大的 1024x1024) 作为基础图像
    # 并将所有其他图像 (16 到 512) 附加进去
    base_image = icns_images.pop(-1)  # 弹出 1024px

    base_image.save(
        output_icns_path,
        format="ICNS",
        append_images=icns_images  # 附加所有其他尺寸
    )

    print(f"icon.icns 已成功生成于: {output_icns_path}")
    print(f"(包含 {len(icns_unique_sizes)} 个图层: {icns_unique_sizes})")

except Exception as e:
    print(f"使用 Pillow 生成 icon.icns 失败: {e}")
    print("请确保您的 Pillow 库是最新版本 (pip install --upgrade pillow)")
