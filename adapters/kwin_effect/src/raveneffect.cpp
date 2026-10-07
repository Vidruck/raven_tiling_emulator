/*
 * raveneffect.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Implementación del efecto nativo de KWin para Raven Tiling Emulator.
 */

#include "raveneffect.h"
#include "dbus/raven_effect_adaptor.h"
#include <effect/effecthandler.h>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QLoggingCategory>
#include <chrono>

Q_LOGGING_CATEGORY(RAVEN_EFFECT, "raven.effect", QtInfoMsg)


/**
 * @brief Constructor de la clase RavenEffect.
 * 
 * Inicializa el adaptador D-Bus y registra el servicio y objeto en el bus de sesión
 * para permitir la comunicación remota (IPC). También conecta las señales del
 * compositor de KWin para la gestión de ventanas si está disponible.
 */
RavenEffect::RavenEffect()
    : m_dbusAdaptor(new RavenEffectAdaptor(this))
{
    QDBusConnection dbus = QDBusConnection::sessionBus();
    if (dbus.registerService(QStringLiteral("org.kde.kwin.RavenEffect"))) {
        if (dbus.registerObject(QStringLiteral("/Effects/Raven"), this)) {
            qCInfo(RAVEN_EFFECT) << "RavenEffect D-Bus service registrado exitosamente en /Effects/Raven";
        } else {
            qCWarning(RAVEN_EFFECT) << "Fallo al registrar objeto D-Bus /Effects/Raven";
        }
    } else {
        qCWarning(RAVEN_EFFECT) << "Fallo al registrar servicio D-Bus org.kde.kwin.RavenEffect (¿servicio duplicado?)";
    }

    if (KWin::effects) {
        connect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        connect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
    }
}

/**
 * @brief Destructor de la clase RavenEffect.
 * 
 * Se encarga de desconectar las señales de KWin y dar de baja el servicio
 * y el objeto D-Bus registrados durante la inicialización.
 */
RavenEffect::~RavenEffect()
{
    if (KWin::effects) {
        disconnect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
    }
    QDBusConnection dbus = QDBusConnection::sessionBus();
    dbus.unregisterObject(QStringLiteral("/Effects/Raven"));
    dbus.unregisterService(QStringLiteral("org.kde.kwin.RavenEffect"));
}

/**
 * @brief Verifica si el efecto está soportado por el compositor actual.
 * 
 * @return true si el compositor está activo y soporta efectos; false en caso contrario.
 */
bool RavenEffect::supported()
{
    return KWin::effects && KWin::effects->compositingType() != KWin::NoCompositing;
}

/**
 * @brief Comprueba si el efecto tiene alguna animación en progreso.
 * 
 * @return true si existe al menos una animación activa; false de lo contrario.
 */
bool RavenEffect::isActive() const
{
    return !m_activeAnimations.isEmpty();
}

/**
 * @brief Maneja el evento de finalización de una animación de KWin.
 * 
 * Este método es llamado por el compositor (KWin) cuando una animación termina.
 * Se utiliza para purgar las animaciones que han concluido del registro interno
 * y liberar memoria.
 * 
 * @param w Puntero a la ventana cuyo atributo terminó de animarse.
 * @param a Atributo que estaba siendo animado.
 * @param meta Metadatos asociados a la animación.
 */
void RavenEffect::animationEnded(KWin::EffectWindow *w, Attribute a, uint meta)
{
    Q_UNUSED(a);
    Q_UNUSED(meta);
    if (!w) {
        return;
    }

    // Purgar entradas huérfanas en m_activeAnimations
    for (auto it = m_activeAnimations.begin(); it != m_activeAnimations.end(); ) {
        if (it.value().isEmpty()) {
            it = m_activeAnimations.erase(it);
        } else {
            ++it;
        }
    }
}

/**
 * @brief Slot que se ejecuta cuando se añade una nueva ventana al compositor.
 * 
 * Ignora ventanas de sistema (paneles, notificaciones, etc.) y lanza una
 * animación reactiva (gelatina / zoom-in) indicando el "nacimiento" de la ventana.
 * Además, notifica al motor backend a través de D-Bus.
 * 
 * @param w Puntero a la nueva ventana añadida.
 */
void RavenEffect::slotWindowAdded(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    // Filtrar ventanas no gestionables, overlays, paneles o popups
    if (w->isDock() || w->isDesktop() || w->isNotification() || w->isTooltip() || !w->isNormalWindow()) {
        return;
    }

    QString windowId = w->internalId().toString();
    if (windowId.isEmpty()) {
        windowId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    }

    qCInfo(RAVEN_EFFECT) << "Ventana detectada en compositing, iniciando nacimiento reactivo:" << windowId;

    // Iniciar animación de nacimiento nativa
    animateWindowBirth(windowId, 140);

    // 2. Emitir señal D-Bus hacia Raven Backend
    if (m_dbusAdaptor) {
        Q_EMIT m_dbusAdaptor->WindowBirthStarted(windowId);
    }
}

/**
 * @brief Slot que se ejecuta cuando una ventana es cerrada.
 * 
 * Cancela inmediatamente cualquier animación activa que pertenezca
 * a la ventana que está siendo destruida para evitar fallos de memoria (segmentation faults).
 * 
 * @param w Puntero a la ventana que se cierra.
 */
void RavenEffect::slotWindowClosed(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    QString windowId = w->internalId().toString();
    if (!windowId.isEmpty()) {
        cancelWindowAnimation(windowId);
    }

    QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    cancelWindowAnimation(hexId);

    QString decId = QString::number(reinterpret_cast<quintptr>(w));
    cancelWindowAnimation(decId);
}


/**
 * @brief Busca una ventana activa (EffectWindow) por su identificador.
 * 
 * Realiza una búsqueda segura en la pila (stack) de ventanas del compositor.
 * Intenta coincidir utilizando el puntero en formato hexadecimal (0x...),
 * el valor decimal o el identificador interno de la ventana (internal ID).
 * 
 * @param windowId Cadena de texto con el identificador de la ventana a buscar.
 * @return KWin::EffectWindow* Puntero a la ventana encontrada, o nullptr si no existe.
 */
KWin::EffectWindow *RavenEffect::findWindowById(const QString &windowId) const
{
    if (!KWin::effects) {
        return nullptr;
    }

    const auto windows = KWin::effects->stackingOrder();
    for (auto *win : windows) {
        if (!win) continue;
        
        // Match con internal ID o address pointer hex string (0x...)
        QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(win), 0, 16);
        if (hexId.compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }

        // Match por windowId decimal o address
        if (QString::number(reinterpret_cast<quintptr>(win)).compare(windowId) == 0) {
            return win;
        }

        // Match con internalId QString si estuviera disponible
        if (win->internalId().toString().compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }
    }
    return nullptr;
}

/**
 * @brief Convierte un nombre de curva de aceleración en un objeto QEasingCurve nativo.
 * 
 * Transforma identificadores en formato cadena (ej. "EaseOutBack" o "Jelly")
 * a la representación nativa de curva de suavizado (easing curve) de Qt.
 * 
 * @param easingName Nombre descriptivo de la curva.
 * @return QEasingCurve Objeto con la curva de aceleración solicitada.
 */
QEasingCurve RavenEffect::parseEasing(const QString &easingName) const
{
    if (easingName == QLatin1String("EaseOutExpo")) {
        return QEasingCurve(QEasingCurve::OutExpo);
    } else if (easingName == QLatin1String("EaseOutCubic")) {
        return QEasingCurve(QEasingCurve::OutCubic);
    } else if (easingName == QLatin1String("EaseOutBack")) {
        return QEasingCurve(QEasingCurve::OutBack);
    } else if (easingName == QLatin1String("EaseOutElastic") || easingName == QLatin1String("Jelly")) {
        // Curva elástica amortiguada
        QEasingCurve elastic(QEasingCurve::OutElastic);
        elastic.setPeriod(0.25);
        elastic.setAmplitude(1.05);
        return elastic;
    } else if (easingName == QLatin1String("EaseInOutQuad")) {
        return QEasingCurve(QEasingCurve::InOutQuad);
    } else if (easingName == QLatin1String("Linear")) {
        return QEasingCurve(QEasingCurve::Linear);
    }
    // Efecto por defecto
    return QEasingCurve(QEasingCurve::OutCubic);
}

/**
 * @brief Interpola de forma animada la geometría (tamaño y posición) de una ventana.
 * 
 * Lanza una animación nativa usando el gestor de efectos para mover y escalar la ventana
 * desde su rectángulo de origen al de destino usando una curva específica.
 * 
 * @param windowId Identificador interno de la ventana.
 * @param startRect Rectángulo inicial (x, y, ancho, alto).
 * @param targetRect Rectángulo objetivo.
 * @param durationMs Duración de la transición en milisegundos.
 * @param easingName Nombre de la curva de suavizado (easing curve) a utilizar.
 */
void RavenEffect::animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                                        int durationMs, const QString &easingName)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelWindowAnimation(windowId);

    // Ajustar duración por defecto si el valor no es válido
    if (durationMs <= 0 || durationMs > 300) {
        durationMs = 130;
    }

    QEasingCurve curve = parseEasing(easingName);

    uint meta = 0;
    setMetaData(SourceAnchor, Anchor::Left | Anchor::Top, meta);

    // 1. Animación de posición (traslación desde startRect hacia targetRect)
    quint64 posAnim = animate(w,
                              KWin::AnimationEffect::Position,
                              meta,
                              std::chrono::milliseconds(durationMs),
                              KWin::FPx2(0.0, 0.0),
                              curve,
                              0,
                              KWin::FPx2(startRect.x() - targetRect.x(), startRect.y() - targetRect.y()));

    if (posAnim != 0) {
        m_activeAnimations[windowId].append(posAnim);
    }

    // 2. Animación de escala
    // Se interpola la escala desde (startW / targetW, startH / targetH) hacia (1.0, 1.0)
    if (targetRect.width() > 0.0 && targetRect.height() > 0.0 &&
        startRect.width() > 0.0 && startRect.height() > 0.0) {
        qreal scaleX = startRect.width() / targetRect.width();
        qreal scaleY = startRect.height() / targetRect.height();

        // Solo disparar animación de escala si las dimensiones difieren perceptiblemente
        if (qAbs(scaleX - 1.0) > 0.005 || qAbs(scaleY - 1.0) > 0.005) {
            quint64 scaleAnim = animate(w,
                                        KWin::AnimationEffect::Scale,
                                        meta,
                                        std::chrono::milliseconds(durationMs),
                                        KWin::FPx2(1.0, 1.0),
                                        curve,
                                        0,
                                        KWin::FPx2(scaleX, scaleY));

            if (scaleAnim != 0) {
                m_activeAnimations[windowId].append(scaleAnim);
            }
        }
    }
}

/**
 * @brief Procesa un bloque (batch) de animaciones de geometría a partir de JSON.
 * 
 * Lee un texto en formato JSON que describe múltiples transiciones y lanza 
 * las animaciones de manera simultánea para todas las ventanas indicadas.
 * Útil para los cambios de disposición (layouts) del gestor de ventanas en mosaico (tiling).
 * 
 * @param jsonPayload Cadena JSON con el arreglo (array) de objetos de animación.
 */
void RavenEffect::animateBatchGeometry(const QString &jsonPayload)
{
    if (jsonPayload.isEmpty() || jsonPayload == QLatin1String("[]") || jsonPayload == QLatin1String("{}")) {
        return;
    }

    QJsonDocument doc = QJsonDocument::fromJson(jsonPayload.toUtf8());
    if (!doc.isObject()) {
        return;
    }

    QJsonObject root = doc.object();
    QJsonArray anims = root.value(QStringLiteral("animations")).toArray();
    if (anims.isEmpty()) {
        return;
    }

    for (const auto &val : anims) {
        if (!val.isObject()) continue;
        QJsonObject obj = val.toObject();

        QString winId = obj.value(QStringLiteral("id")).toString();
        int durationMs = obj.value(QStringLiteral("duration_ms")).toInt(250);
        QString easing = obj.value(QStringLiteral("easing")).toString(QStringLiteral("EaseOutBack"));

        QJsonArray fromArr = obj.value(QStringLiteral("from")).toArray();
        QJsonArray toArr = obj.value(QStringLiteral("to")).toArray();

        if (fromArr.size() >= 4 && toArr.size() >= 4) {
            QRectF from(fromArr[0].toDouble(), fromArr[1].toDouble(), fromArr[2].toDouble(), fromArr[3].toDouble());
            QRectF to(toArr[0].toDouble(), toArr[1].toDouble(), toArr[2].toDouble(), toArr[3].toDouble());
            animateWindowGeometry(winId, from, to, durationMs, easing);
        }
    }
}

/**
 * @brief Realiza la animación inicial de nacimiento (aparición) de una ventana.
 * 
 * Ejecuta una interpolación doble combinando el escalado desde un tamaño menor 
 * hacia el tamaño natural (1.0), junto con una transición de opacidad (fade in).
 * 
 * @param windowId Identificador de la ventana.
 * @param durationMs Duración de la animación de nacimiento en milisegundos.
 */
void RavenEffect::animateWindowBirth(const QString &windowId, int durationMs)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelWindowAnimation(windowId);

    if (durationMs <= 0) {
        durationMs = 250;
    }

    QEasingCurve curve(QEasingCurve::OutElastic);
    QEasingCurve opacityCurve(QEasingCurve::OutCubic);

    uint metaCenter = 0;
    setMetaData(SourceAnchor, Anchor::Horizontal | Anchor::Vertical, metaCenter);

    // Animación de escala desde 0.85 a 1.0
    quint64 scaleAnim = animate(w,
                                KWin::AnimationEffect::Scale,
                                metaCenter,
                                std::chrono::milliseconds(durationMs),
                                KWin::FPx2(1.0, 1.0),
                                curve,
                                0,
                                KWin::FPx2(0.85, 0.85));

    // Animación de opacidad (Fade-in desde 0.0 a 1.0)
    uint metaOpacity = 0;
    quint64 opacityAnim = animate(w,
                                  KWin::AnimationEffect::Opacity,
                                  metaOpacity,
                                  std::chrono::milliseconds(static_cast<int>(durationMs * 0.7)),
                                  KWin::FPx2(1.0, 1.0),
                                  opacityCurve,
                                  0,
                                  KWin::FPx2(0.0, 0.0));

    if (scaleAnim != 0) m_activeAnimations[windowId].append(scaleAnim);
    if (opacityAnim != 0) m_activeAnimations[windowId].append(opacityAnim);
}

/**
 * @brief Cancela inmediatamente todas las animaciones activas asociadas a una ventana.
 * 
 * Útil cuando se requiere interrumpir el efecto visual debido a un cerrado
 * súbito de la ventana u otra acción que vuelva obsoleta la animación.
 * 
 * @param windowId Identificador de la ventana.
 */
void RavenEffect::cancelWindowAnimation(const QString &windowId)
{
    if (m_activeAnimations.contains(windowId)) {
        const auto anims = m_activeAnimations.take(windowId);
        for (quint64 id : anims) {
            cancel(id);
        }
    }
}

#include "moc_raveneffect.cpp"
