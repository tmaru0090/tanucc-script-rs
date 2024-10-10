"""
import cv2
import pygetwindow as gw
import pyautogui
import numpy as np
import time
import keyboard
from enum import Enum

class IrisuButton(Enum):
    LClick = 1
    RClick = 2
    Esc = 3

def get_window_data(window_title):
    windows = gw.getWindowsWithTitle(window_title)
    if not windows:
        raise Exception(f"ウィンドウ '{window_title}' が見つかりません")
    window = windows[0]
    width = window.width
    height = window.height
    center_x = window.left + width // 2
    center_y = window.top + height // 2
    return window, width, height, center_x, center_y

def irisu_get_title_pos(center_x, center_y):
    title_menu = [
        (center_x, center_y + 20),
        (center_x + 100, center_y + 100),
        (center_x + 150, center_y + 150),
        (center_x, center_y - 150),
    ]
    return title_menu

def press_button(button_type, x, y, start_sleep_time, up_sleep_time):
    time.sleep(start_sleep_time)
    if button_type == IrisuButton.LClick:
        pyautogui.mouseDown(x, y)
        time.sleep(up_sleep_time)
        pyautogui.mouseUp(x, y)
    elif button_type == IrisuButton.RClick:
        pyautogui.mouseDown(button='right', x=x, y=y)
        time.sleep(up_sleep_time)
        pyautogui.mouseUp(button='right', x=x, y=y)
    elif button_type == IrisuButton.Esc:
        pyautogui.keyDown('esc')
        time.sleep(up_sleep_time)
        pyautogui.keyUp('esc')

# 指定のexeのウィンドウタイトルを取得
window_title = "irisu syndrome"  # ここに実際のウィンドウタイトルを入力

# ウィンドウデータを取得
window, width, height, center_x, center_y = get_window_data(window_title)

# タイトル画面の座標を取得
title_positions = irisu_get_title_pos(center_x, center_y)

# タイトル項目の最初をクリック
press_button(IrisuButton.LClick, title_positions[0][0], title_positions[0][1], 1, 0.1)
time.sleep(3)  # 数秒待機

paused = False

while True:
    # キー入力をチェック
    if keyboard.is_pressed('p'):
        paused = not paused
        time.sleep(0.5)  # 状態が切り替わるのを防ぐために少し待機

    if paused:
        # キャプチャー画面を表示
        screenshot = pyautogui.screenshot(region=(window.left, window.top, window.width, window.height))
        frame = np.array(screenshot)
        frame = cv2.cvtColor(frame, cv2.COLOR_RGB2BGR)
        cv2.imshow('Paused - Press P to Resume', frame)
        cv2.waitKey(1)
        while paused:
            if keyboard.is_pressed('p'):
                paused = False
                time.sleep(0.5)  # 状態が切り替わるのを防ぐために少し待機
                cv2.destroyWindow('Paused - Press P to Resume')
                break
        continue

    # ウィンドウをフォーカス
    if not window.isActive:
        window.activate()
        time.sleep(0.1)  # ウィンドウがアクティブになるまで少し待機

    # ウィンドウのスクリーンショットを取得
    screenshot = pyautogui.screenshot(region=(window.left, window.top, window.width, window.height))
    
    # スクリーンショットをnumpy配列に変換
    frame = np.array(screenshot)
    
    # BGRに変換（OpenCVはBGRを使用）
    frame = cv2.cvtColor(frame, cv2.COLOR_RGB2BGR)
    
    # 白い枠を認識
    gray_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
    _, thresh = cv2.threshold(gray_frame, 200, 255, cv2.THRESH_BINARY)
    
    # 輪郭を検出
    contours, _ = cv2.findContours(thresh, cv2.RETR_TREE, cv2.CHAIN_APPROX_SIMPLE)
    
    for contour in contours:
        # 輪郭の周囲に多角形を近似
        approx = cv2.approxPolyDP(contour, 0.02 * cv2.arcLength(contour, True), True)
        
        # 細長い白い枠を検出
        x, y, w, h = cv2.boundingRect(approx)
        aspect_ratio = float(w) / h
        if 0.1 < aspect_ratio < 10.0 and w > 50 and h > 50:  # アスペクト比とサイズが適切な範囲内であることを確認
            cv2.drawContours(frame, [approx], 0, (0, 255, 0), 2)
            print(f"Detected white frame at: x={x}, y={y}, w={w}, h={h}")
    
    # フレームをウィンドウに表示
    cv2.imshow('Window Capture', frame)
    
    # 'q'キーが押されたらループを終了
    if cv2.waitKey(1) & 0xFF == ord('q'):
        break

    # 少し待機してから次のフレームを処理
    time.sleep(0.1)

# リソースを解放
cv2.destroyAllWindows()
"""



import cv2
import face_recognition
import os
import pickle
import numpy as np

data_folder = 'face_data'
if not os.path.exists(data_folder):
    os.makedirs(data_folder)

def save_face_data(face_id, face_encoding):
    # face_encodingをそのまま保存する (128次元)
    with open(os.path.join(data_folder, f'face_{face_id}.pkl'), 'wb') as f:
        pickle.dump(face_encoding, f)

def load_face_data():
    face_encodings = {}
    for filename in os.listdir(data_folder):
        if filename.endswith('.pkl'):
            with open(os.path.join(data_folder, filename), 'rb') as f:
                face_id = filename.split('_')[1].split('.')[0]
                # 保存されたエンコーディングが128次元であることを保証
                face_encoding = pickle.load(f)
                if face_encoding.shape == (128,):  # 128次元か確認
                    face_encodings[face_id] = face_encoding
                else:
                    print(f"Warning: Face encoding for {face_id} has incorrect shape {face_encoding.shape}")
    return face_encodings

cap = cv2.VideoCapture(0)
face_encodings = load_face_data()

face_id_counter = len(face_encodings)

while True:
    ret, frame = cap.read()
    if not ret:
        break

    rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    face_locations = face_recognition.face_locations(rgb_frame)
    face_encodings_in_frame = face_recognition.face_encodings(rgb_frame, face_locations)

    for (top, right, bottom, left), face_encoding in zip(face_locations, face_encodings_in_frame):
        matched = False
        for saved_id, saved_encoding in face_encodings.items():
            match = face_recognition.compare_faces([saved_encoding], face_encoding, tolerance=0.6)
            if match[0]:
                matched = True
                cv2.putText(frame, f'ID: {saved_id}', (left, top - 10), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 255, 0), 2)
                break
        
        if not matched:
            cv2.putText(frame, 'New Face', (left, top - 10), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 0, 255), 2)
            save_face_data(face_id_counter, face_encoding)
            face_encodings[str(face_id_counter)] = face_encoding  # 新しく保存した顔のエンコーディングをメモリに追加
            face_id_counter += 1

        cv2.rectangle(frame, (left, top), (right, bottom), (255, 0, 0), 2)

    cv2.imshow('Face Detection', frame)

    if cv2.waitKey(1) & 0xFF == ord('q'):
        break

cap.release()
cv2.destroyAllWindows()
